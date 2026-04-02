import { useState, useEffect } from 'react';
import TeamCell from './TeamCell';
import RuneBuildBox from './RuneBuildBox';
import { useLCU } from '../../contexts/LCUContext';
import { getChampName } from '../../utils/lolDisplay';

const PHASE_TRANSLATE: Record<string, string> = {
    'PLANNING': 'Déclaration',
    'BAN_PICK': 'Ban / Pick',
    'FINALIZATION': 'Préparation',
    'GAME_STARTING': 'Lancement'
};

const LobbyTab = () => {
    const { 
        myChamp, champs, builds, runesData, 
        lobbyMyTeam, lobbyTheirTeam, lobbyState, 
        isLoadingBuilds, appData
    } = useLCU();

    const [simMode] = useState(false);

    const getRuneIcon = (id: number) => {
        for (const tree of runesData) {
            for (const slot of tree.slots) {
                const rune = slot.runes.find((r: any) => r.id === id);
                if (rune) return rune.icon;
            }
        }
        return '';
    };

    // --- LOGIQUE DE SIMULATION INTERNE ---
    const simBuilds: any[] = [
        {
            name: "Méta Standard",
            winrate: "54.2%",
            banrate: "12%",
            primaryStyleId: 8100,
            subStyleId: 8000,
            perkIds: [8112, 8139, 8138, 8105, 8009, 8014],
            shards: [5008, 5008, 5002],
            spells: [4, 14]
        },
        {
            name: "Contre Akali",
            winrate: "56.1%",
            banrate: "22%",
            primaryStyleId: 8100,
            subStyleId: 8400,
            perkIds: [8112, 8143, 8138, 8106, 8473, 8451],
            shards: [5008, 5008, 5003],
            spells: [4, 14],
            counters: [
                { name: "Vex", keystoneId: 8112 },
                { name: "Fizz", keystoneId: 8112 },
                { name: "Pantheon", keystoneId: 8010 }
            ]
        },
        {
            name: "Scaling Late",
            winrate: "51.5%",
            banrate: "5%",
            primaryStyleId: 8000,
            subStyleId: 8300,
            perkIds: [8010, 9111, 9104, 8299, 8345, 8347],
            shards: [5005, 5008, 5001],
            spells: [4, 12]
        }
    ];

    const simMyTeam = [
        { summonerName: 'Allié 1', assignedPosition: 'top', championId: 266, cellId: 0 },
        { summonerName: 'Allié 2', assignedPosition: 'jungle', championId: 64, cellId: 1 },
        { summonerName: 'KCorp Laoy#KCB', assignedPosition: 'middle', championId: 517, cellId: 2, puuid: 'me' },
        { summonerName: 'Allié 4', assignedPosition: 'bottom', championId: 222, cellId: 3 },
        { summonerName: 'Allié 5', assignedPosition: 'utility', championId: 111, cellId: 4 }
    ];

    const simTheirTeam = [
        { summonerName: 'Ennemi 1', assignedPosition: 'top', championId: 122 },
        { summonerName: 'Ennemi 2', assignedPosition: 'jungle', championId: 121 },
        { summonerName: 'Ennemi 3', assignedPosition: 'middle', championId: 84 },
        { summonerName: 'Ennemi 4', assignedPosition: 'bottom', championId: 81 },
        { summonerName: 'Ennemi 5', assignedPosition: 'utility', championId: 53 }
    ];

    const activeBuilds = simMode ? simBuilds : builds;
    const activeMyTeam = simMode ? simMyTeam : lobbyMyTeam;
    const activeTheirTeam = simMode ? simTheirTeam : lobbyTheirTeam;
    const activeCounters = (simMode ? simBuilds[1].counters : builds.find(b => b.counters)?.counters) || [];
    const activeChampId = simMode ? 517 : myChamp;

    const [scanKey, setScanKey] = useState(0);

    useEffect(() => {
        if (activeChampId !== 0) {
            setScanKey(prev => prev + 1);
        }
    }, [activeChampId]);

    const getSimChampName = (id: number) => {
        if (id === 517) return "Sylas";
        return getChampName(id, champs);
    };

    const getSimChampAlias = (id: number, name?: string) => {
        const c = champs.find(x => x.id === id || (name && x.name.toLowerCase() === name.toLowerCase()));
        if (c) return c.alias;
        if (id === 517 || name === "Sylas") return "Sylas";
        if (id === 266) return "Aatrox";
        if (id === 64) return "LeeSin";
        if (id === 222) return "Jinx";
        if (id === 111) return "Nautilus";
        if (id === 122) return "Darius";
        if (id === 121) return "Khazix";
        if (id === 84) return "Akali";
        if (id === 81) return "Ezreal";
        if (id === 53) return "Blitzcrank";
        if (name === "Vex") return "Vex";
        if (name === "Fizz") return "Fizz";
        if (name === "Pantheon") return "Pantheon";
        return "Unknown";
    };

    return (
        <div className="w-full h-full flex flex-col overflow-hidden bg-[#050507]/40 backdrop-blur-sm relative">
            {activeChampId !== 0 && <div key={scanKey} className="scanning-line" />}
            
            {/* Top Section: Team Draft (Portraits + Bans) - Now anchored to the Top */}
            <div className="w-full bg-[#0a0a0c]/60 border-b border-white/5 pt-4 pb-2 px-6 shrink-0 relative z-20 shadow-2xl">
                <div className="w-full max-w-[1400px] mx-auto flex justify-between items-start gap-6">
                    {/* Blue Team Side */}
                    <div className="flex flex-1 items-start gap-3 min-w-0">
                        <div className="flex flex-1 gap-1.5 min-w-0 overflow-hidden">
                            {activeMyTeam.length > 0 ? activeMyTeam.map((p, i) => (
                                <TeamCell key={i} p={p} isBlue={true} forceMockMe={simMode && p.cellId === 2} />
                            )) : [1, 2, 3, 4, 5].map(i => <div key={i} className="flex-1 min-w-0 max-w-[5.2rem] aspect-[20/28] h-auto bg-white/5 border border-white/5 rounded-sm" />)}
                        </div>

                        {/* Blue Team Bans */}
                        <div className="flex flex-col gap-1 shrink-0 pt-1">
                            <span className="text-[6px] text-blue-500/40 font-black uppercase tracking-widest text-center">Bans</span>
                            <div className="flex gap-1">
                                {[0, 1, 2, 3, 4].map(idx => {
                                    const ban = lobbyState?.actions?.flat().find((a: any) => a.type === 'ban' && a.actorCellId === idx && a.completed);
                                    return (
                                        <div key={idx} className="w-6 h-6 bg-black/40 border border-white/10 flex items-center justify-center grayscale opacity-60 overflow-hidden rounded shadow-inner shrink-0">
                                            {ban && ban.championId > 0 && (
                                                <img src={`https://ddragon.leagueoflegends.com/cdn/${lobbyState?.v || '15.5.1'}/img/champion/${champs.find((c: any) => c.id === ban.championId)?.alias || getChampName(ban.championId, champs)}.png`} className="w-full h-full object-cover" alt="" />
                                            )}
                                        </div>
                                    );
                                })}
                            </div>
                        </div>
                    </div>

                    {/* Center: Timer & Phase Info */}
                    <div className="text-center px-4 shrink-0 flex flex-col items-center justify-center gap-1 min-w-[180px] h-full pt-2">
                        <div className="text-4xl font-black text-white font-mono tracking-tighter drop-shadow-xl leading-none">
                            {simMode ? '30' : (lobbyState?.timer?.displayTime !== undefined ? Math.max(0, Math.floor(lobbyState.timer.displayTime / 1000)) : (lobbyState?.timer?.adjustedTimeLeftInPhase ? Math.max(0, Math.floor(lobbyState.timer.adjustedTimeLeftInPhase / 1000)) : '--'))}
                        </div>
                        <div className="text-[8px] text-red-500/80 font-black uppercase tracking-[0.3em] mt-1 bg-red-500/5 border border-red-500/20 px-3 py-1 rounded-full shadow-[0_0_15px_rgba(239,68,68,0.1)]">
                            {simMode ? 'BAN / PICK' : (PHASE_TRANSLATE[lobbyState?.timer?.phase] || lobbyState?.timer?.phase || 'ATTENTE')}
                        </div>
                    </div>

                    {/* Red Team Side */}
                    <div className="flex flex-1 flex-row-reverse items-start gap-3 min-w-0">
                        <div className="flex flex-1 flex-row-reverse gap-1.5 min-w-0 overflow-hidden">
                            {activeTheirTeam.length > 0 ? activeTheirTeam.map((p, i) => (
                                <TeamCell key={i} p={p} isBlue={false} />
                            )) : [1, 2, 3, 4, 5].map(i => <div key={i} className="flex-1 min-w-0 max-w-[5.2rem] aspect-[20/28] h-auto bg-white/5 border border-white/5 rounded-sm" />)}
                        </div>

                        {/* Red Team Bans */}
                        <div className="flex flex-col gap-1 shrink-0 pt-1">
                            <span className="text-[6px] text-red-500/40 font-black uppercase tracking-widest text-center">Bans</span>
                            <div className="flex gap-1 flex-row-reverse">
                                {[5, 6, 7, 8, 9].map(idx => {
                                    const ban = lobbyState?.actions?.flat().find((a: any) => a.type === 'ban' && a.actorCellId === idx && a.completed);
                                    return (
                                        <div key={idx} className="w-6 h-6 bg-black/40 border border-white/10 flex items-center justify-center grayscale opacity-60 overflow-hidden rounded shadow-inner shrink-0">
                                            {ban && ban.championId > 0 && (
                                                <img src={`https://ddragon.leagueoflegends.com/cdn/${lobbyState?.v || '15.5.1'}/img/champion/${champs.find((c: any) => c.id === ban.championId)?.alias || getChampName(ban.championId, champs)}.png`} className="w-full h-full object-cover" alt="" />
                                            )}
                                        </div>
                                    );
                                })}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Middle/Bottom Section: Champion Info + AI Builds Analysis */}
            <div className="flex-1 overflow-y-auto px-6 py-4 animate-in fade-in slide-in-from-bottom-4 duration-700">
                <div className="w-full max-w-7xl mx-auto h-full flex flex-col">
                    <div className="border-b border-white/5 pb-2 mb-4 flex justify-between items-end">
                        <div className="flex flex-col">
                            <span className="text-red-500/60 text-[9px] font-black uppercase tracking-[0.4em] mb-1">
                                {simMode ? 'Aperçu Simulation' : 'Champion Actuellement Sélectionné'}
                            </span>
                            <h2 key={activeChampId} className="text-3xl font-black text-white uppercase tracking-tighter leading-none flex items-center gap-4">
                                {getSimChampName(activeChampId)}
                                {isLoadingBuilds && <span className="text-[10px] text-red-500 animate-pulse font-bold bg-red-500/5 px-2 py-1 rounded border border-red-500/20 normal-case tracking-normal">AI Analyzing Meta...</span>}
                            </h2>
                        </div>
                        
                        {/* Draft Warnings / Counters anchor */}
                        {(appData?.draftWarnings !== false) && activeCounters && activeCounters.length > 0 && (
                            <div className="flex flex-col items-end gap-1.5 animate-in fade-in slide-in-from-right-4 duration-1000">
                                <span className="text-[8px] text-red-500 font-black uppercase tracking-[0.2em] bg-red-500/10 px-2 py-0.5 border border-red-500/20 rounded">Analyse du Draft : Counters Suggérés</span>
                                <div className="flex gap-3">
                                    {activeCounters.map((cug: any, idx: number) => {
                                        const alias = getSimChampAlias(0, cug.name);
                                        const rIcon = getRuneIcon(cug.keystoneId);
                                        return alias ? (
                                            <div key={idx} className="group relative">
                                                <div className="relative">
                                                    <img src={`https://ddragon.leagueoflegends.com/cdn/${lobbyState?.v || '15.5.1'}/img/champion/${alias}.png`} className="w-10 h-10 border border-red-500/40 rounded-lg shadow-2xl transition-all duration-300 group-hover:scale-110 group-hover:border-red-500" alt={cug.name} />
                                                    {rIcon && (
                                                        <div className="absolute -bottom-1 -right-1 w-5 h-5 bg-black rounded-full border border-red-500/60 flex items-center justify-center p-1 shadow-2xl">
                                                            <img src={`https://ddragon.leagueoflegends.com/cdn/img/${rIcon}`} className="w-full h-full object-contain" alt="Keystone" />
                                                        </div>
                                                    )}
                                                </div>
                                            </div>
                                        ) : null;
                                    })}
                                </div>
                            </div>
                        )}
                    </div>
                    
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-6 flex-1 pt-2 pb-8">
                        {activeBuilds.map((b, i) => (
                            <RuneBuildBox key={i} b={b} i={i} />
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
};

export default LobbyTab;
