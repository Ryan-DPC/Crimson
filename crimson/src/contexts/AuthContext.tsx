import React, { createContext, useContext, useEffect, useState } from 'react';
import type { Session, User } from '@supabase/supabase-js';
import { supabase } from '../lib/supabase';
import { invoke } from '@tauri-apps/api/core';

interface AuthContextType {
    session: Session | null;
    user: User | null;
    isPremium: boolean;
    loading: boolean;
    signOut: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType>({
    session: null,
    user: null,
    isPremium: false,
    loading: true,
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

    const checkPremiumStatus = async (userId?: string) => {
        if (!userId) {
            setIsPremium(false);
            setLoading(false);
            return;
        }

        try {
            const { data, error } = await supabase
                .from('profiles')
                .select('is_premium, premium_token')
                .eq('id', userId)
                .single();

            if (error) throw error;
            
            // Check robust token rule
            let hasValidPremium = false;
            if (data?.is_premium && data?.premium_token && data.premium_token.length >= 64) {
                const numberCount = (data.premium_token.match(/\d/g) || []).length;
                if (numberCount >= 5) {
                    hasValidPremium = true;
                }
            }
            
            setIsPremium(hasValidPremium);

            // Start the backend server now that we are authenticated
            try {
                await invoke('crimson_start_server');
            } catch (e) {
                console.error('Failed to start server:', e);
            }
        } catch (error) {
            console.error('Error fetching premium status:', error);
            setIsPremium(false);
        } finally {
            setLoading(false);
        }
    };

    const signOut = async () => {
        try {
            await invoke('crimson_stop_server');
        } catch (e) {
            console.error('Failed to stop server:', e);
        }
        await supabase.auth.signOut();
    };

    return (
        <AuthContext.Provider value={{ session, user, isPremium, loading, signOut }}>
            {children}
        </AuthContext.Provider>
    );
};

export const useAuth = () => useContext(AuthContext);
