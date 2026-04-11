// App State
const actionContexts = {
    "com.laoy.streamdock.crimson.autoaccept": [],
    "com.laoy.streamdock.crimson.dodge": [],
    "com.laoy.streamdock.crimson.display": [],
    "com.laoy.streamdock.crimson.autoban": [],
    "com.laoy.streamdock.crimson.autopick": [],
    "com.laoy.streamdock.crimson.inject": [],
    "com.laoy.streamdock.crimson.infoboard": []
};

let gameState = {
    phase: "None",
    rank: { tier: "UNRANKED", division: "", lp: 0 },
    champion: 0,
    championName: "",
    autoAccept: false,
    autoBan: null,
    autoPick: null,
    serverConnected: false
};

// ==========================================
// 1. BACKEND LISTENER (Logic Layer)
// ==========================================
api.onMessage = (data) => {
    console.log("Crimson Message Received:", data.type, data);
    switch (data.type) {
        case 'AUTO_ACCEPT_STATE':
            gameState.autoAccept = data.enabled;
            updateContexts("com.laoy.streamdock.crimson.autoaccept", data.enabled ? 1 : 0);
            refreshAllDisplays(); // Dynamic text update
            break;
        case 'GAME_PHASE':
            gameState.phase = data.phase;
            refreshAllDisplays();
            break;
        case 'CHAMP_SELECT':
            gameState.champion = data.championId;
            gameState.championName = data.championName || "";
            refreshAllDisplays();
            break;
        case 'RANK_UPDATE':
            gameState.rank = data;
            refreshAllDisplays();
            break;
        case 'AUTO_BAN_STATE':
            gameState.autoBan = data.championId;
            updateContexts("com.laoy.streamdock.crimson.autoban", data.championId ? 1 : 0);
            refreshAllDisplays();
            break;
        case 'AUTO_PICK_STATE':
            gameState.autoPick = data.championId;
            updateContexts("com.laoy.streamdock.crimson.autopick", data.championId ? 1 : 0);
            refreshAllDisplays();
            break;
    }
};

api.onStatusChange = (isConnected) => {
    gameState.serverConnected = isConnected;
    updateContexts("com.laoy.streamdock.crimson.infoboard", isConnected ? 1 : 0);
    refreshAllDisplays();
};

// Heartbeat Polling (Every 3 minutes as requested)
setInterval(() => {
    // If we want to force a visual pulse or a re-check
    if (gameState.serverConnected) {
        console.log("Heartbeat: Server is healthy.");
        refreshAllDisplays();
    }
}, 3 * 60 * 1000);


// ==========================================
// 2. UI RENDERER GLUE
// ==========================================
function updateContexts(action, stateId) {
    const contexts = actionContexts[action] || [];
    contexts.forEach(context => ui.setState(context, stateId));
}

function refreshAllDisplays() {
    Object.keys(actionContexts).forEach(action => {
        actionContexts[action].forEach(context => updateDisplayLogic(context, action));
    });
}

function updateDisplayLogic(context, action, controller = "Keypad") {
    let title = "";
    
    switch (action) {
        case "com.laoy.streamdock.crimson.autoaccept":
            title = gameState.autoAccept ? "ACTIVE" : "OFF";
            break;
        case "com.laoy.streamdock.crimson.display":
            if (gameState.phase === "ChampSelect") {
                title = gameState.championName || "DRAFT";
            } else {
                title = gameState.rank.tier !== "UNRANKED" ? `${gameState.rank.tier}\n${gameState.rank.lp}LP` : "RANK";
            }
            break;
        case "com.laoy.streamdock.crimson.dodge":
            if (gameState.phase === "ChampSelect" || gameState.phase === "Lobby") {
                title = "DODGE";
            }
            break;
        case "com.laoy.streamdock.crimson.autoban":
            title = gameState.autoBan ? "BANNING" : "AUTO";
            break;
        case "com.laoy.streamdock.crimson.autopick":
            title = gameState.autoPick ? "PICKING" : "AUTO";
            break;
        case "com.laoy.streamdock.crimson.infoboard":
            title = gameState.serverConnected ? "LINKED" : "OFFLINE";
            break;
    }
    
    ui.setTitle(context, title);
}


// ==========================================
// 3. ELGATO PLUGIN REGISTRATION
// ==========================================
window.connectElgatoStreamDeckSocket = function(inPort, inPluginUUID, inRegisterEvent, inInfo) {
    const streamDeckSocket = new WebSocket("ws://127.0.0.1:" + inPort);
    ui.setSocket(streamDeckSocket);

    streamDeckSocket.onopen = function () {
        streamDeckSocket.send(JSON.stringify({ "event": inRegisterEvent, "uuid": inPluginUUID }));
    };

    streamDeckSocket.onmessage = function (evt) {
        const jsonObj = JSON.parse(evt.data);
        const event = jsonObj['event'];
        const action = jsonObj['action'];
        const context = jsonObj['context'];
        const controller = jsonObj['controller'];

        if (event === "willAppear") {
            if (actionContexts[action] && !actionContexts[action].includes(context)) {
                actionContexts[action].push(context);
            }
            
            // Set initial state for toggles
            if (action === "com.laoy.streamdock.crimson.autoaccept") {
                ui.setState(context, gameState.autoAccept ? 1 : 0);
            } else if (action === "com.laoy.streamdock.crimson.infoboard") {
                ui.setState(context, gameState.serverConnected ? 1 : 0);
            }
            
            updateDisplayLogic(context, action, controller);
        }

        if (event === "willDisappear") {
            if (actionContexts[action]) {
                actionContexts[action] = actionContexts[action].filter(c => c !== context);
            }
        }

        if (event === "keyDown") {
            handleActionClick(action);
        }
    };
};


// ==========================================
// 4. ELGATO INPUT ROUTER
// ==========================================
function handleActionClick(action) {
    switch (action) {
        case "com.laoy.streamdock.crimson.autoaccept":
            api.send({ type: 'TOGGLE_AUTO_ACCEPT' });
            break;
        case "com.laoy.streamdock.crimson.dodge":
            api.send({ type: 'DODGE_GAME' });
            break;
        case "com.laoy.streamdock.crimson.autoban":
            api.send({ type: 'TOGGLE_AUTO_BAN', championId: 0 });
            break;
        case "com.laoy.streamdock.crimson.autopick":
            api.send({ type: 'TOGGLE_AUTO_PICK', championId: 0 });
            break;
        case "com.laoy.streamdock.crimson.inject":
            api.send({ 
                type: 'INJECT_BUILD', 
                championId: gameState.champion,
                championName: gameState.championName
            });
            break;
        case "com.laoy.streamdock.crimson.infoboard":
            // Optional: clicking Info Board could attempt a manual reconnect
            if (!gameState.serverConnected) api.connect();
            break;
    }
}
