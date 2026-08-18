import React, { createContext, useContext, useEffect, useState } from 'react';
import type { Session, User } from '@supabase/supabase-js';
import { supabase } from '../lib/supabase';
import { invoke } from '@tauri-apps/api/core';

interface AuthContextType {
    session: Session | null;
    user: User | null;
    isPremium: boolean;
    loading: boolean;
    refreshPremium: () => Promise<boolean>;
    signOut: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType>({
    session: null,
    user: null,
    isPremium: false,
    loading: true,
    refreshPremium: async () => false,
    signOut: async () => {},
});

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [session, setSession] = useState<Session | null>(null);
    const [user, setUser] = useState<User | null>(null);
    const [isPremium, setIsPremium] = useState(false);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        // Initial session fetch
        supabase.auth.getSession().then(({ data: { session } }) => {
            setSession(session);
            setUser(session?.user ?? null);
            checkPremiumStatus(session?.user?.id);
        });

        // Listen for auth changes
        const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
            setSession(session);
            setUser(session?.user ?? null);
            checkPremiumStatus(session?.user?.id);
        });

        return () => subscription.unsubscribe();
    }, []);

    const checkPremiumStatus = async (userId?: string): Promise<boolean> => {
        if (!userId) {
            setIsPremium(false);
            setLoading(false);
            return false;
        }

        // Start the sidecar as soon as we have a Supabase user. Premium lookup
        // must not gate this: a profiles RLS/network failure used to leave the
        // backend permanently down while the UI looked "logged in".
        try {
            await invoke('crimson_start_server');
        } catch (e) {
            console.error('Failed to start server:', e);
        }

        try {
            const { data, error } = await supabase
                .from('profiles')
                .select('is_premium, premium_token')
                .eq('id', userId)
                .single();

            if (error) throw error;

            // Sert uniquement a l'affichage. L'ancienne heuristique sur la
            // longueur du jeton ne prouvait rien : c'est la base qui fait foi,
            // et le serveur local revalide de son cote avant toute commande.
            const premium = data?.is_premium === true;
            setIsPremium(premium);
            return premium;
        } catch (error) {
            console.error('Error fetching premium status:', error);
            setIsPremium(false);
            return false;
        } finally {
            setLoading(false);
        }
    };

    /** Re-lit is_premium apres un achat — pas besoin de reinstaller. */
    const refreshPremium = async (): Promise<boolean> => {
        const { data: { session: current } } = await supabase.auth.getSession();
        return checkPremiumStatus(current?.user?.id);
    };

    const signOut = async () => {
        // Explicit logout — clear sidecar refresh so StreamDock does not stay premium.
        try {
            let token = await invoke<string | null>('crimson_get_auth_token').catch(() => null);
            const url = token
                ? `ws://127.0.0.1:40510/?token=${encodeURIComponent(token)}`
                : 'ws://127.0.0.1:40510/';
            await new Promise<void>((resolve) => {
                try {
                    const ws = new WebSocket(url);
                    const done = () => resolve();
                    ws.onopen = () => {
                        ws.send(JSON.stringify({ type: 'AUTH_LOGOUT' }));
                        ws.close();
                        done();
                    };
                    ws.onerror = done;
                    setTimeout(done, 1500);
                } catch {
                    resolve();
                }
            });
        } catch (e) {
            console.error('Failed to notify server of logout:', e);
        }
        try {
            await invoke('crimson_stop_server');
        } catch (e) {
            console.error('Failed to stop server:', e);
        }
        await supabase.auth.signOut();
    };

    return (
        <AuthContext.Provider value={{ session, user, isPremium, loading, refreshPremium, signOut }}>
            {children}
        </AuthContext.Provider>
    );
};

export const useAuth = () => useContext(AuthContext);
