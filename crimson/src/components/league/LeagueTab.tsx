import { useState } from 'react';
import LobbyTab from '../lobby/LobbyTab';
import HistoryTab from '../history/HistoryTab';

export default function LeagueTab({ initialTab = 'lobby' }: { initialTab?: string }) {
  const [subTab, setSubTab] = useState(initialTab);

  return (
    <div className="h-full flex flex-col overflow-hidden animate-in fade-in slide-in-from-bottom-4 duration-1000">
      <div className="flex justify-center mt-6 shrink-0 z-10">
        <div className="flex items-center bg-white/5 p-1 rounded-xl border border-white/5">
          <button 
            onClick={() => setSubTab('lobby')}
            className={`px-8 py-2.5 rounded-lg text-xs font-black tracking-widest uppercase transition-all duration-300 ${subTab === 'lobby' ? 'bg-white/10 text-white shadow-lg shadow-black/50' : 'text-white/40 hover:text-white/70'}`}
          >
            LOBBY
          </button>
          <div className="w-px h-4 bg-white/10 mx-1" />
          <button 
            onClick={() => setSubTab('history')}
            className={`px-8 py-2.5 rounded-lg text-xs font-black tracking-widest uppercase transition-all duration-300 ${subTab === 'history' ? 'bg-white/10 text-white shadow-lg shadow-black/50' : 'text-white/40 hover:text-white/70'}`}
          >
            HISTORY
          </button>
        </div>
      </div>
      
      <div className="flex-1 min-h-0 flex flex-col mt-4">
        {subTab === 'lobby' ? <LobbyTab /> : (
          <div className="flex-1 min-h-0 overflow-y-auto scrollbar-hide max-w-[1600px] mx-auto p-8 w-full">
            <HistoryTab />
          </div>
        )}
      </div>
    </div>
  );
}
