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
        isLoadingBuilds 
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
        <div className="w-full h-full flex flex-col justify-between overflow-hidden bg-[#050507]/40 backdrop-blur-sm relative">
            {activeChampId !== 0 && <div key={scanKey} className="scanning-line" />}
            {/* Top Section: Header + Builds */}
            <div className="w-full flex-1 flex flex-col pt-3 px-4 min-h-0">
                <div className="w-full max-w-6xl mx-auto flex flex-col h-full">
                    <div className="border-b border-white/5 pb-1 mb-2 flex justify-between items-end shrink-0">
                        <div className="flex flex-col">
                            <span className="text-red-500 text-[8px] font-black uppercase tracking-[0.3em] mb-0.5">
                                {simMode ? 'Aperçu Simulation' : 'Champion Actuel'}
                            </span>
                            <h2 key={activeChampId} className="text-xl font-bold text-white uppercase tracking-widest leading-none animate-in fade-in slide-in-from-bottom-2 duration-700">
                                {getSimChampName(activeChampId)}
                                {isLoadingBuilds && <span className="ml-3 text-[10px] text-red-500 animate-pulse tracking-tighter normal-case">Analyse AI en cours...</span>}
                            </h2>
                        </div>
                        <div className="flex items-center gap-3">
                            <span className="text-white/30 text-[10px] font-bold uppercase tracking-widest">
                                {simMode ? 'MODE SIMULATION' : (lobbyState?.timer?.phase ? PHASE_TRANSLATE[lobbyState.timer.phase] || lobbyState.timer.phase : 'Attente LCU...')}
                            </span>
                        </div>
                    </div>
                    
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-3 flex-1 min-h-0 pt-1 pb-2">
                        {activeBuilds.map((b, i) => (
                            <RuneBuildBox 
                                key={i} b={b} i={i} 
                            />
                        ))}
                    </div>
                </div>
            </div>

            {/* Bottom Section: Footer anchored */}
            <div className="w-full bg-[#0a0a0c]/90 border-t border-white/5 py-1 px-4 shrink-0">
                <div className="w-full max-w-[1400px] mx-auto flex justify-between items-end gap-2 pt-1 pb-0.5">
                    {/* Blue Team Side */}
                    <div className="flex flex-1 items-end gap-2 min-w-0">
                        <div className="flex flex-1 gap-1 min-w-0 overflow-hidden">
                            {activeMyTeam.length > 0 ? activeMyTeam.map((p, i) => (
                                <TeamCell key={i} p={p} isBlue={true} forceMockMe={simMode && p.cellId === 2} />
                            )) : [1, 2, 3, 4, 5].map(i => <div key={i} className="flex-1 min-w-0 max-w-[4.8rem] aspect-[20/28] h-auto bg-[#111115] border border-white/5" />)}
                        </div>

                        {/* Blue Team Bans (Inner) */}
                        <div className="flex gap-0.5 shrink-0 mb-0.5 pb-0.5">
                            {[0, 1, 2, 3, 4].map(idx => {
                                const ban = lobbyState?.actions?.flat().find((a: any) => a.type === 'ban' && a.actorCellId === idx && a.completed);
                                return (
                                    <div key={idx} className="w-6 h-6 bg-[#111115] border border-red-500/10 flex items-center justify-center grayscale opacity-60 overflow-hidden rounded-sm shrink-0">
                                        {ban && ban.championId > 0 && (
                                            <img src={`https://ddragon.leagueoflegends.com/cdn/${lobbyState?.v || '15.5.1'}/img/champion/${champs.find((c: any) => c.id === ban.championId)?.alias || getChampName(ban.championId, champs)}.png`} className="w-full h-full object-cover" alt="" />
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    </div>

                    {/* Timer & Counters Center */}
                    <div className="text-center px-2 shrink-0 flex flex-col items-center gap-1 min-w-[140px]">
                        {activeCounters && activeCounters.length > 0 && (
                            <div className="flex flex-col items-center gap-1 animate-in fade-in slide-in-from-top-2 duration-500">
                                <span className="text-[6px] text-red-500/60 font-black uppercase tracking-[0.2em] whitespace-nowrap">Counters Suggérés</span>
                                <div className="flex justify-center gap-2">
                                    {activeCounters.map((cug: any, idx: number) => {
                                        const alias = getSimChampAlias(0, cug.name);
                                        const rIcon = getRuneIcon(cug.keystoneId);
                                        return alias ? (
                                            <div key={idx} className="group relative">
                                                <div className="relative">
                                                    <img src={`https://ddragon.leagueoflegends.com/cdn/${lobbyState?.v || '15.5.1'}/img/champion/${alias}.png`} className="w-8 h-8 border border-red-500/30 rounded shadow-lg shadow-red-500/10 group-hover:scale-110 group-hover:border-red-500 transition-all duration-300" alt={cug.name} />
                                                    {rIcon && (
                                                        <div className="absolute -bottom-1 -right-1 w-4 h-4 bg-black/90 rounded-full border border-red-500/40 flex items-center justify-center p-0.5 shadow-xl z-20">
                                                            <img src={`https://ddragon.leagueoflegends.com/cdn/img/${rIcon}`} className="w-full h-full object-contain" alt="Keystone" />
                                                        </div>
                                                    )}
                                                </div>
                                                <div className="absolute -bottom-4 left-1/2 -translate-x-1/2 opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap text-[6px] font-black text-white bg-red-600 px-1 rounded pointer-events-none z-30">{cug.name}</div>
                                            </div>
                                        ) : null;
                                    })}
                                </div>
                            </div>
                        )}
                        <div className="flex flex-col items-center">
                            <div className="text-2xl font-bold text-neutral-100 font-mono tracking-tighter drop-shadow-[0_0_8px_rgba(255,255,255,0.15)] leading-none">
                                {simMode ? '30' : (lobbyState?.timer?.displayTime !== undefined ? Math.max(0, Math.floor(lobbyState.timer.displayTime / 1000)) : (lobbyState?.timer?.adjustedTimeLeftInPhase ? Math.max(0, Math.floor(lobbyState.timer.adjustedTimeLeftInPhase / 1000)) : '--'))}
                            </div>
                            <div className="text-[5px] text-white/20 font-black uppercase tracking-[0.25em] mt-0.5 bg-white/5 px-1 py-0.5 rounded-full">{simMode ? 'BAN / PICK' : (PHASE_TRANSLATE[lobbyState?.timer?.phase] || lobbyState?.timer?.phase || 'ATTENTE')}</div>
                        </div>
                    </div>

                    {/* Red Team Side */}
                    <div className="flex flex-1 flex-row-reverse items-end gap-2 min-w-0">
                        <div className="flex flex-1 flex-row-reverse gap-1 min-w-0 overflow-hidden">
                            {activeTheirTeam.length > 0 ? activeTheirTeam.map((p, i) => (
                                <TeamCell key={i} p={p} isBlue={false} />
                            )) : [1, 2, 3, 4, 5].map(i => <div key={i} className="flex-1 min-w-0 max-w-[4.8rem] aspect-[20/28] h-auto bg-[#111115] border border-white/5" />)}
                        </div>

                        {/* Red Team Bans (Inner) */}
                        <div className="flex gap-0.5 shrink-0 flex-row-reverse mb-0.5 pb-0.5">
                            {[5, 6, 7, 8, 9].map(idx => {
                                const ban = lobbyState?.actions?.flat().find((a: any) => a.type === 'ban' && a.actorCellId === idx && a.completed);
                                return (
                                    <div key={idx} className="w-6 h-6 bg-[#111115] border border-red-500/10 flex items-center justify-center grayscale opacity-60 overflow-hidden rounded-sm shrink-0">
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
    );
};

export default LobbyTab;
