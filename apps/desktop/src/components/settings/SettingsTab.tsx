import { useState, useEffect } from 'react';
import { check } from '@tauri-apps/plugin-updater';
import { useLCU } from '../../contexts/LCUContext';
import { Settings as SettingsIcon, Shield, Bell, Eye, Cpu, Check, Loader2, RefreshCw, Download } from 'lucide-react';

const SettingsTab = () => {
    const { appData, updateGeminiKey } = useLCU();
    const [keyInput, setKeyInput] = useState(appData?.geminiApiKey || '');
    const [status, setStatus] = useState<'idle' | 'loading' | 'success'>('idle');
    const [isEditingKey, setIsEditingKey] = useState(!appData?.geminiApiKey);

    // --- Update state ---
    const [updateStatus, setUpdateStatus] = useState<'idle' | 'checking' | 'up-to-date' | 'available' | 'installing'>('idle');
    const [availableVersion, setAvailableVersion] = useState<string | null>(null);

    useEffect(() => {
        if (appData?.geminiApiKey) {
            setKeyInput(appData.geminiApiKey);
            setIsEditingKey(false);
        }
    }, [appData?.geminiApiKey]);

    const handleVerify = () => {
        setStatus('loading');
        updateGeminiKey(keyInput);
        setTimeout(() => {
            setStatus('success');
            setTimeout(() => setStatus('idle'), 3000);
        }, 800);
    };

    const handleCheckUpdate = async () => {
        setUpdateStatus('checking');
        try {
            const update = await check();
            if (update) {
                setAvailableVersion(update.version);
                setUpdateStatus('available');
            } else {
                setUpdateStatus('up-to-date');
                setTimeout(() => setUpdateStatus('idle'), 4000);
            }
        } catch {
            setUpdateStatus('idle');
        }
    };

    const handleInstallUpdate = async () => {
        setUpdateStatus('installing');
        try {
            const update = await check();
            if (update) {
                await update.downloadAndInstall();
            }
        } catch {
            setUpdateStatus('available');
        }
    };

    return (
        <div className="w-full max-w-4xl mx-auto space-y-8 mt-12 px-8 animate-in fade-in duration-700">
            <div className="flex items-center gap-4 mb-8">
                <div className="p-3 bg-red-600/10 rounded-2xl border border-red-500/20">
                    <SettingsIcon className="w-6 h-6 text-red-500" />
                </div>
                <div>
                    <h2 className="text-2xl font-black text-white uppercase tracking-widest">Paramètres</h2>
                    <p className="text-white/30 text-xs font-bold uppercase tracking-widest mt-1">Configuration globale de Crimson</p>
                </div>
            </div>

            <div className="grid grid-cols-1 gap-6">
                {/* AI Configuration */}
                <div className="bg-[#111115] border border-white/5 p-8 rounded-2xl shadow-2xl relative overflow-hidden group">
                    <div className="absolute top-0 right-0 p-4 opacity-5">
                        <Cpu className="w-24 h-24" />
                    </div>
                    <div className="relative z-10">
                        <div className="flex items-center gap-3 mb-6">
                            <Shield className={`w-5 h-5 ${!isEditingKey ? 'text-green-500' : 'text-red-500'}`} />
                            <h3 className="text-sm font-black text-white uppercase tracking-widest">Intelligence Artificielle</h3>
                        </div>
                        <p className="text-white/40 text-[10px] font-bold uppercase tracking-widest mb-6 leading-relaxed max-w-md">
                            Configurez votre clé API Gemini pour bénéficier des analyses de draft en temps réel et des suggestions de runes dynamiques.
                        </p>
                        <div className="flex flex-col gap-2 max-w-lg">
                            <label className="text-[9px] text-white/20 uppercase font-black tracking-widest mb-1">Google Gemini API Key</label>
                                {!isEditingKey ? (
                                    <div className="flex items-center gap-3 w-full bg-green-500/10 border border-green-500/20 px-4 py-3 rounded-xl transition-all cursor-pointer group" onClick={() => setIsEditingKey(true)}>
                                        <div className="flex-1 text-green-500/80 text-xs font-mono tracking-widest">••••••••••••••••••••••••••••••••••</div>
                                        <div className="flex items-center gap-2 text-green-500">
                                            <span className="text-[10px] font-black uppercase tracking-widest group-hover:text-green-400">Opérationnelle</span>
                                            <Check className="w-4 h-4 group-hover:scale-110 transition-transform" />
                                        </div>
                                    </div>
                                ) : (
                                    <div className="flex gap-3 w-full">
                                        <input 
                                            type="password"
                                            value={keyInput}
                                            onChange={(e) => setKeyInput(e.target.value)}
                                            placeholder="COLLEZ VOTRE CLÉ ICI..."
                                            className="flex-1 bg-black/40 border border-white/5 px-4 py-3 text-xs font-mono focus:border-red-500/50 outline-none text-white/50 transition-all rounded-xl"
                                        />
                                        <button 
                                            onClick={handleVerify}
                                            disabled={status === 'loading' || !keyInput}
                                            className={`px-6 flex items-center justify-center gap-2 text-[10px] font-black uppercase tracking-widest rounded-xl transition-all ${
                                                status === 'success' ? 'bg-green-600/20 text-green-500 border border-green-500/30' : 
                                                'bg-white/5 hover:bg-white/10 border border-white/10 text-white/60'
                                            }`}
                                        >
                                            {status === 'loading' && <Loader2 className="w-4 h-4 animate-spin" />}
                                            {status === 'success' && <Check className="w-4 h-4" />}
                                            {status === 'idle' && "Vérifier"}
                                            {status !== 'idle' && status !== 'success' && "Vérifier"}
                                        </button>
                                    </div>
                                )}
                        </div>
                    </div>
                </div>

                {/* Updates */}
                <div className="bg-[#111115] border border-white/5 p-8 rounded-2xl shadow-2xl">
                    <div className="flex items-center gap-3 mb-6">
                        <RefreshCw className={`w-5 h-5 text-red-500 ${updateStatus === 'checking' || updateStatus === 'installing' ? 'animate-spin' : ''}`} />
                        <h3 className="text-sm font-black text-white uppercase tracking-widest">Mises à Jour</h3>
                        <span className="ml-auto text-[9px] font-black text-white/20 uppercase tracking-widest">v0.1.0</span>
                    </div>

                    <div className="flex items-center justify-between">
                        <div>
                            {updateStatus === 'idle' && <p className="text-white/30 text-[10px] uppercase font-bold tracking-widest">Vérifiez si une nouvelle version est disponible.</p>}
                            {updateStatus === 'checking' && <p className="text-white/50 text-[10px] uppercase font-bold tracking-widest animate-pulse">Vérification en cours...</p>}
                            {updateStatus === 'up-to-date' && <p className="text-green-500 text-[10px] uppercase font-black tracking-widest">✓ Vous êtes à jour !</p>}
                            {updateStatus === 'available' && <p className="text-red-400 text-[10px] uppercase font-black tracking-widest">🚀 Version <span className="text-white">{availableVersion}</span> disponible !</p>}
                            {updateStatus === 'installing' && <p className="text-blue-400 text-[10px] uppercase font-black tracking-widest animate-pulse">Téléchargement... L'app va redémarrer automatiquement.</p>}
                        </div>
                        <div className="flex gap-3">
                            {updateStatus === 'available' && (
                                <button
                                    onClick={handleInstallUpdate}
                                    className="flex items-center gap-2 px-5 py-2.5 bg-red-600 hover:bg-red-500 text-white text-[10px] font-black uppercase tracking-widest rounded-xl transition-all shadow-[0_0_15px_rgba(239,68,68,0.3)]"
                                >
                                    <Download className="w-3.5 h-3.5" />
                                    Installer
                                </button>
                            )}
                            {(updateStatus === 'idle' || updateStatus === 'up-to-date' || updateStatus === 'checking') && (
                                <button
                                    onClick={handleCheckUpdate}
                                    disabled={updateStatus === 'checking'}
                                    className="flex items-center gap-2 px-5 py-2.5 bg-white/5 hover:bg-white/10 border border-white/10 hover:border-white/20 text-white/60 hover:text-white text-[10px] font-black uppercase tracking-widest rounded-xl transition-all"
                                >
                                    <RefreshCw className={`w-3.5 h-3.5 ${updateStatus === 'checking' ? 'animate-spin' : ''}`} />
                                    Vérifier
                                </button>
                            )}
                        </div>
                    </div>
                </div>

                {/* Notifications & Prefs */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div className="bg-[#111115] border border-white/5 p-8 rounded-2xl shadow-2xl">
                        <div className="flex items-center gap-3 mb-6">
                            <Bell className="w-5 h-5 text-red-500" />
                            <h3 className="text-sm font-black text-white uppercase tracking-widest">Notifications</h3>
                        </div>
                        <div className="space-y-4">
                            <div className="flex items-center justify-between">
                                <span className="text-[10px] font-bold text-white/60 uppercase tracking-widest">Auto-Accept Alert</span>
                                <div className="w-10 h-5 bg-red-600 rounded-full relative">
                                    <div className="absolute right-1 top-1 w-3 h-3 bg-white rounded-full shadow-sm" />
                                </div>
                            </div>
                            <div className="flex items-center justify-between opacity-50">
                                <span className="text-[10px] font-bold text-white/60 uppercase tracking-widest">Draft Warnings</span>
                                <div className="w-10 h-5 bg-white/10 rounded-full relative">
                                    <div className="absolute left-1 top-1 w-3 h-3 bg-white/40 rounded-full" />
                                </div>
                            </div>
                        </div>
                    </div>

                    <div className="bg-[#111115] border border-white/5 p-8 rounded-2xl shadow-2xl">
                        <div className="flex items-center gap-3 mb-6">
                            <Eye className="w-5 h-5 text-red-500" />
                            <h3 className="text-sm font-black text-white uppercase tracking-widest">Interface</h3>
                        </div>
                        <div className="space-y-4">
                            <div className="flex items-center justify-between">
                                <span className="text-[10px] font-bold text-white/60 uppercase tracking-widest">Dark Glass Mode</span>
                                <div className="w-10 h-5 bg-red-600 rounded-full relative">
                                    <div className="absolute right-1 top-1 w-3 h-3 bg-white rounded-full shadow-sm" />
                                </div>
                            </div>
                            <div className="flex items-center justify-between">
                                <span className="text-[10px] font-bold text-white/60 uppercase tracking-widest">Animations Reduites</span>
                                <div className="w-10 h-5 bg-white/10 rounded-full relative">
                                    <div className="absolute left-1 top-1 w-3 h-3 bg-white/40 rounded-full" />
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default SettingsTab;
