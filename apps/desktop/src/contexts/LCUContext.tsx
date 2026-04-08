import React, { createContext, useContext, useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Summoner, Match, RuneBuild, RadarResult } from '../types';
import { getChampName } from '../utils/lolDisplay';
import runesDataJson from '../assets/data/runesData.json';

interface LCUContextType {
    sum: Summoner | null;
    lobbyState: any;
    lobbyMyTeam: any[];
    lobbyTheirTeam: any[];
    radar: RadarResult[];
    gamePhase: string;
    rank: { tier: string, division: string, lp: number, tftTier: string, tftDivision: string, tftLp: number };
    hist: Match[];
    champs: any[];
    runesData: any[];
    v: string;
    myChamp: number;
    enemyMid: string | null;
    isLoadingBuilds: boolean;
    builds: (RuneBuild | null)[];
    isImporting: number | null;
    appData: any;
    
    // Actions
    setTab: (tab: string) => void;
    toggleSimMode: () => void;
    simMode: boolean;
    tab: string;
    toggleAutoBan: (id: number) => Promise<void>;
    toggleAutoPick: (id: number) => Promise<void>;
    updateGeminiKey: (key: string) => Promise<void>;
    updateSetting: (key: string, value: boolean) => Promise<void>;
    doImport: (build: RuneBuild, index: number) => Promise<void>;
    handleSecondaryClick: (buildIndex: number, runeId: number, slotIndex: number) => void;
    handleShardClick: (buildIndex: number, rIdx: number, shardId: number) => void;
}

const LCUContext = createContext<LCUContextType | undefined>(undefined);

const ROLE_TRANSLATE: Record<string, string> = {
    'top': 'top',
    'jungle': 'jungle',
    'middle': 'mid',
    'bottom': 'adc',
    'utility': 'support',
    '': ''
};

export const LCUProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [sum, setSum] = useState<Summoner | null>(null);
    const [lobbyState, setLobbyState] = useState<any>(null);
    const [lobbyMyTeam, setLobbyMyTeam] = useState<any[]>([]);
    const [lobbyTheirTeam, setLobbyTheirTeam] = useState<any[]>([]);
    const [radar, setRadar] = useState<RadarResult[]>([]);
    const [gamePhase, setGamePhase] = useState<string>('None');
    const [rank, setRank] = useState({ tier: 'UNRANKED', division: '', lp: 0, tftTier: 'UNRANKED', tftDivision: '', tftLp: 0 });
    const [hist, setHist] = useState<Match[]>([]);
    const [champs, setChamps] = useState<any[]>([]);
    const [v, setV] = useState('15.5.1');
    const [myChamp, setMyChamp] = useState<number>(0);
    const [enemyMid, setEnemyMid] = useState<string | null>(null);
    const [isLoadingBuilds, setIsLoadingBuilds] = useState(false);

    const [isImporting, setIsImporting] = useState<number | null>(null);
    const [simMode, setSimMode] = useState(false);
    const [tab, setTab] = useState('home');
    const [appData, setAppData] = useState<any>(null);

    const scannedLobbyId = useRef<string>('');
    const lastFetchParams = useRef<string>('');
    const hasFetchedInitialState = useRef<boolean>(false);
    const champsRef = useRef<any[]>([]);

    // --- UTILS ---
    const fetchAllyRadar = async (team: any[], currentLobbyId: string) => {
        if (scannedLobbyId.current === currentLobbyId) return;
        scannedLobbyId.current = currentLobbyId;
        
        const radarResults: RadarResult[] = [];
        for (const p of team) {
            if (!p.puuid || p.cellId === lobbyState?.localPlayerCellId) continue;
            try {
                const hStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: `/lol-match-history/v1/products/lol/${p.puuid}/matches?begIndex=0&endIndex=20`, body: null });
                if (!hStr) continue;
                const h = JSON.parse(hStr);
                const games = h?.games?.games || (Array.isArray(h) ? h : []);
                if (games.length > 0) {
                    let wins = 0;
                    let validGames = 0;
                    let lastResults: boolean[] = [];

                    games.slice(0, 10).forEach((g: any) => {
                        const isRemake = g.gameDuration < 300 || g.gameDuration === 0 || g.endOfGameResult === 'Abort_Unexpected';
                        if (!isRemake) {
                            validGames++;
                            const isWin = g.participants?.[0]?.stats?.win ?? g.stats?.win ?? false;
                            if (isWin) wins++;
                            lastResults.push(isWin);
                        }
                    });
                    
                    const winrate = validGames > 0 ? Math.round((wins / validGames) * 100) : null;
                    const lossStreak = lastResults.indexOf(true) === -1 ? lastResults.length : lastResults.indexOf(true);
                    const winStreak = lastResults.indexOf(false) === -1 ? lastResults.length : lastResults.indexOf(false);

                    const isTrollPick = (p.assignedPosition === 'JUNGLE' && (p.championId === 350 || p.championId === 16)) ||
                                       (p.assignedPosition === 'TOP' && p.championId === 350);

                    radarResults.push({
                        puuid: p.puuid,
                        winrate,
                        games: validGames,
                        isTilt: lossStreak >= 3 || (winrate !== null && winrate < 40),
                        isSmurf: winStreak >= 5 || (winrate !== null && winrate > 70),
                        isTroll: isTrollPick,
                        lastResults
                    });
                }
            } catch { }
        }
        setRadar(radarResults);
    };

    const fetchHistory = async (accountId: number, puuid: string) => {
        let found = false;
        const endpoints = [
            `/lol-match-history/v1/products/lol/${puuid}/matches?begIndex=0&endIndex=10`,
            `/lol-match-history/v3/matchlist/account/${accountId}?begIndex=0&endIndex=10`,
            `/lol-match-history/v1/recent-matches`
        ];
        for (const ep of endpoints) {
            if (found) break;
            try {
                const hStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: ep, body: null });
                if (!hStr) continue;
                const h = JSON.parse(hStr);
                const games = h?.games?.games || (Array.isArray(h) ? h : []);
                
                if (games.length > 0) {
                    const latestHist = games.slice(0, 10).map((g: any) => ({
                        gameId: g.gameId,
                        gameCreation: g.gameCreation,
                        championId: g.participants?.[0]?.championId || g.championId,
                        stats: g.participants?.[0]?.stats || g.stats,
                        gameQueueId: g.gameQueueId,
                        gameDuration: g.gameDuration
                    }));
                    setHist(latestHist); 
                    found = true;

                    latestHist.forEach((m: any) => {
                        invoke('insert_match', { m: {
                            game_id: m.gameId,
                            timestamp: m.gameCreation || 0,
                            champion_id: m.championId,
                            kills: m.stats?.kills || 0,
                            deaths: m.stats?.deaths || 0,
                            assists: m.stats?.assists || 0,
                            win: m.stats?.win || false,
                            queue_id: m.gameQueueId || 0,
                            game_duration: m.gameDuration || 0
                        }});
                    });

                    const d = await invoke<any>('get_app_data');
                    d.hist = latestHist;
                    await invoke('set_app_data', { data: d });
                    setAppData(d);
                }
            } catch {}
        }

        try {
            const allMatches = await invoke<any[]>('get_all_matches');
            if (allMatches && allMatches.length > 0) {
                setHist(allMatches.map(m => ({
                    gameId: m.game_id,
                    gameCreation: m.timestamp,
                    championId: m.champion_id,
                    stats: { kills: m.kills, deaths: m.deaths, assists: m.assists, win: m.win },
                    gameQueueId: m.queue_id,
                    gameDuration: m.game_duration
                })));
            }
        } catch {}
    };

    const fastPoll = async () => {
        try {
            // Always load appData first regardless of LCU connection state
            const data = await invoke<any>('get_app_data');
            setAppData(data);
        } catch { }

        try {
            const info = await invoke<any>('get_lcu_info');
            
            if (!info || (typeof info === 'object' && !info.port)) {
                setSum(null);
                setGamePhase('None');
                hasFetchedInitialState.current = false;
                return;
            }

            if (!hasFetchedInitialState.current) {
                hasFetchedInitialState.current = true;
                
                try {
                    const phaseStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: '/lol-gameflow/v1/gameflow-phase', body: null });
                    const phase = JSON.parse(phaseStr);
                    setGamePhase(phase);

                    if (phase === 'ChampSelect') {
                        const csStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: '/lol-champ-select/v1/session', body: null });
                        const cs = JSON.parse(csStr);
                        if (cs && cs.timer) cs.timer.localSyncTime = Date.now();
                        setLobbyState(cs);
                        
                        if (cs?.myTeam) {
                            setLobbyMyTeam([...cs.myTeam].sort((a, b) => (a.cellId || 0) - (b.cellId || 0)));
                            setLobbyTheirTeam([...(cs.theirTeam || [])].sort((a, b) => (a.cellId || 0) - (b.cellId || 0)));
                            const me = cs.myTeam.find((p: any) => p.cellId === cs.localPlayerCellId);
                            if (me) setMyChamp(me.championId || me.championPickIntent);
                            
                            if (cs?.chatDetails?.multiUserChatId) {
                                fetchAllyRadar(cs.myTeam, cs.chatDetails.multiUserChatId);
                            }

                            if (cs.theirTeam && cs.theirTeam.length > 0) {
                                const oppMid = cs.theirTeam.find((p: any) => p.assignedPosition === 'middle' || p.assignedPosition === 'mid');
                                if (oppMid && oppMid.championId) setEnemyMid(getChampName(oppMid.championId, champs));
                            }
                        }
                    } else if (phase === 'InProgress') {
                        const gameInfoStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: '/lol-gameflow/v1/session', body: null });
                        const gameInfo = JSON.parse(gameInfoStr);
                        if (gameInfo?.gameData?.playerChampionId) {
                            setMyChamp(gameInfo.gameData.playerChampionId);
                        }
                    }
                } catch { }
            }

            if (!sum || !sum.puuid) {
                try {
                    const sStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: '/lol-summoner/v1/current-summoner', body: null });
                    const s = JSON.parse(sStr);
                    if (s && s.puuid) {
                        setSum(s);
                        fetchHistory(s.accountId, s.puuid);
                    }
                } catch { }
            }
            
            // Re-fetch radar or gameflow slightly less aggressively if myChamp is still 0 (fallback)
            if (myChamp === 0 && hasFetchedInitialState.current) {
                try {
                    const gameInfoStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: '/lol-gameflow/v1/session', body: null });
                    const gameInfo = JSON.parse(gameInfoStr);
                    if (gameInfo?.gameData?.playerChampionId) {
                        setMyChamp(gameInfo.gameData.playerChampionId);
                    }
                } catch {}
            }
        } catch { setSum(null); }
    };

    const loadStatic = async () => {
        try {
            const verStr = await invoke<string>('fetch_ddragon_url', { url: 'https://ddragon.leagueoflegends.com/api/versions.json' });
            const versions = JSON.parse(verStr);
            if (versions && versions[0]) {
                setV(versions[0]);
                
                const cReqStr = await invoke<string>('fetch_ddragon_url', { url: `https://ddragon.leagueoflegends.com/cdn/${versions[0]}/data/fr_FR/champion.json` });
                const cData = JSON.parse(cReqStr);
                const champsArray = Object.values(cData.data).map((c: any) => ({
                    id: parseInt(c.key),
                    name: c.name,
                    alias: c.id
                }));
                // Sort alphabetically by name
                champsArray.sort((a: any, b: any) => a.name.localeCompare(b.name));
                setChamps(champsArray);
                champsRef.current = champsArray;
            }
        } catch (err) { console.error("Static data fetch error:", err); }
    };

    // --- EFFECTS ---
    useEffect(() => {
        loadStatic();
        const i = setInterval(fastPoll, 2000);

        const ws = new WebSocket('ws://127.0.0.1:40509');
        ws.onmessage = async (event) => {
            const msg = JSON.parse(event.data);
            if (msg.type === 'GAME_PHASE') {
                setGamePhase(msg.phase);
            } else if (msg.type === 'CHAMP_SELECT_UPDATE') {
                const cs = msg.data;
                if (cs && cs.timer) cs.timer.localSyncTime = Date.now();
                setLobbyState(cs);
                
                if (cs?.myTeam) {
                    setLobbyMyTeam([...cs.myTeam].sort((a, b) => (a.cellId || 0) - (b.cellId || 0)));
                    setLobbyTheirTeam([...(cs.theirTeam || [])].sort((a, b) => (a.cellId || 0) - (b.cellId || 0)));
                    const me = cs.myTeam.find((p: any) => p.cellId === cs.localPlayerCellId);
                    if (me) setMyChamp(me.championId || me.championPickIntent);
                    
                    if (cs?.chatDetails?.multiUserChatId) {
                        fetchAllyRadar(cs.myTeam, cs.chatDetails.multiUserChatId);
                    }

                    // Enemy Mid Detection
                    if (cs.theirTeam && cs.theirTeam.length > 0) {
                        const oppMid = cs.theirTeam.find((p: any) => p.assignedPosition === 'middle' || p.assignedPosition === 'mid');
                        if (oppMid && oppMid.championId) {
                            setEnemyMid(getChampName(oppMid.championId, champsRef.current));
                        }
                    }
                }
            } else if (msg.type === 'RANK_UPDATE') {
                setRank({ 
                    tier: msg.tier, division: msg.division, lp: msg.lp,
                    tftTier: msg.tft_tier, tftDivision: msg.tft_division, tftLp: msg.tft_lp
                });
            }
        };

        const timerInterval = setInterval(() => {
            setLobbyState((prev: any) => {
                if (!prev || !prev.timer || !prev.timer.adjustedTimeLeftInPhase || !prev.timer.localSyncTime) return prev;
                const elapsedSinceLastSync = Date.now() - prev.timer.localSyncTime;
                return {
                    ...prev,
                    timer: { ...prev.timer, displayTime: Math.max(0, prev.timer.adjustedTimeLeftInPhase - elapsedSinceLastSync) }
                };
            });
        }, 100);

        return () => {
            clearInterval(i);
            clearInterval(timerInterval);
            ws.close();
        };
    }, []);

    useEffect(() => {
        if (simMode) {
            setRadar([
                { puuid: 'test-1', winrate: 30, games: 10, isTilt: true, isSmurf: false, isTroll: false, lastResults: [false] },
                { puuid: 'test-4', winrate: 80, games: 10, isTilt: false, isSmurf: true, isTroll: false, lastResults: [true] }
            ]);
        } else {
            setRadar([]);
        }
    }, [simMode]);

    const [builds, setBuilds] = useState<(RuneBuild | null)[]>([]);

    useEffect(() => {
        const cname = getChampName(simMode ? 517 : myChamp, champs);
        if (!cname || cname === 'Inconnu' || cname === '') {
            if (myChamp === 0 && !simMode) setBuilds([]);
            return;
        }

        const fetchRunes = async () => {
            const me = lobbyMyTeam.find(p => p.cellId === lobbyState?.localPlayerCellId);
            const role = me ? ROLE_TRANSLATE[me.assignedPosition] || 'mid' : 'mid';
            const currentParams = `${cname}-${role}-${enemyMid || 'none'}`;
            if (currentParams === lastFetchParams.current) return;
            lastFetchParams.current = currentParams;

            // Reset builds to 3 null slots for loading placeholders
            setBuilds([null, null, null]);
            setIsLoadingBuilds(true);
            
            // Execute with a slight stagger (1.5s) to avoid Gemini API burst limits for free tiers
            const fetchOne = async (index: number, delayMs: number) => {
                await new Promise(resolve => setTimeout(resolve, delayMs));
                try {
                    const b = await invoke<RuneBuild>('fetch_single_build', { 
                        championName: cname, role: role, opponent: enemyMid || null, patch: v, index 
                    });
                    setBuilds(prev => {
                        const next = [...prev];
                        next[index - 1] = b;
                        return next;
                    });
                } catch (e) {
                    console.error(`Build ${index} fetch error`, e);
                }
            };

            Promise.all([
                fetchOne(1, 0),
                fetchOne(2, 1500),
                fetchOne(3, 3000)
            ]).finally(() => {
                setIsLoadingBuilds(false);
            });
        };
        fetchRunes();
    }, [myChamp, simMode, enemyMid, champs, lobbyMyTeam, lobbyState, v]);

    // --- ACTIONS ---
    const toggleAutoBan = async (id: number) => {
        const d = await invoke<any>('get_app_data');
        d.autoBan = d.autoBan === id ? null : id;
        if (d.autoBan === id && d.autoPick === id) d.autoPick = null;
        await invoke('set_app_data', { data: d });
        setAppData(d);
    };

    const toggleAutoPick = async (id: number) => {
        const d = await invoke<any>('get_app_data');
        d.autoPick = d.autoPick === id ? null : id;
        if (d.autoPick === id && d.autoBan === id) d.autoBan = null;
        await invoke('set_app_data', { data: d });
        setAppData(d);
    };

    const updateGeminiKey = async (key: string) => {
        const d = await invoke<any>('get_app_data');
        d.geminiApiKey = key;
        await invoke('set_app_data', { data: d });
        setAppData(d);
    };

    const updateSetting = async (key: string, value: boolean) => {
        const d = await invoke<any>('get_app_data');
        d[key] = value;
        await invoke('set_app_data', { data: d });
        setAppData(d);
    };

    const doImport = async (build: RuneBuild, index: number) => {
        if (!build || build.primaryStyleId === 0) return;
        setIsImporting(index);
        try {
            await invoke('lcu_request', { method: 'PATCH', endpoint: '/lol-champ-select/v1/session/my-selection', body: JSON.stringify({ spell1Id: build.spells[0], spell2Id: build.spells[1] }) });
            const finalPerks = [...(build.perkIds || []), ...(build.shards || [])].slice(0, 9);
            while(finalPerks.length < 9) finalPerks.push(5001);

            // Delete CRIMSON pages
            const pagesStr = await invoke<string>('lcu_request', { method: 'GET', endpoint: '/lol-perks/v1/pages', body: null });
            const pages = JSON.parse(pagesStr);
            for (const p of pages.filter((p: any) => p.isEditable && p.name.startsWith("CRIMSON:"))) {
                await invoke('lcu_request', { method: 'DELETE', endpoint: `/lol-perks/v1/pages/${p.id}`, body: null });
            }

            await invoke('lcu_request', {
                method: 'POST', endpoint: '/lol-perks/v1/pages', body: JSON.stringify({
                    name: `CRIMSON: ${getChampName(myChamp, champs)}`,
                    primaryStyleId: build.primaryStyleId,
                    subStyleId: build.subStyleId,
                    selectedPerkIds: finalPerks,
                    current: true
                })
            });
            setTimeout(() => setIsImporting(null), 1500);
        } catch (e) { console.error("Import error", e); setIsImporting(null); }
    };

    const handleSecondaryClick = (buildIndex: number, runeId: number, slotIndex: number) => {
        setBuilds(prev => {
            const next = JSON.parse(JSON.stringify(prev));
            const b = next[buildIndex];
            b.perkIds[slotIndex === 0 ? 4 : 5] = runeId;
            return next;
        });
    };

    const handleShardClick = (buildIndex: number, rIdx: number, shardId: number) => {
        setBuilds(prev => {
            const next = JSON.parse(JSON.stringify(prev));
            next[buildIndex].shards[rIdx] = shardId;
            return next;
        });
    };

    const toggleSimMode = () => setSimMode(prev => !prev);

    return (
        <LCUContext.Provider value={{
            sum, lobbyState, lobbyMyTeam, lobbyTheirTeam, radar, gamePhase, rank, hist, champs, runesData: runesDataJson, v, myChamp, enemyMid, isLoadingBuilds, builds, isImporting, appData,
            setTab, toggleSimMode, simMode, tab, toggleAutoBan, toggleAutoPick, updateGeminiKey, updateSetting, doImport, handleSecondaryClick, handleShardClick
        }}>
            {children}
        </LCUContext.Provider>
    );
};

export const useLCU = () => {
    const context = useContext(LCUContext);
    if (!context) throw new Error('useLCU must be used within a LCUProvider');
    return context;
};
