// App State
const actionContexts = {
    "com.laoy.streamdock.crimson.autoaccept": [],
    "com.laoy.streamdock.crimson.dodge": [],
    "com.laoy.streamdock.crimson.display": [],
    "com.laoy.streamdock.crimson.autoban": [],
    "com.laoy.streamdock.crimson.autopick": [],
    "com.laoy.streamdock.crimson.inject": [],
    "com.laoy.streamdock.crimson.global": []
};

let gameState = {
    phase: "None",
    rank: { tier: "", division: "", lp: 0 },
    champion: 0,
    championName: ""
};

// ==========================================
// 1. BACKEND LISTENER (Logic Layer)
// ==========================================
api.onMessage = (data) => {
    switch (data.type) {
        case 'AUTO_ACCEPT_STATE':
            updateContexts("com.laoy.streamdock.crimson.autoaccept", data.enabled ? 1 : 0);
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
            updateContexts("com.laoy.streamdock.crimson.autoban", data.championId ? 1 : 0);
            updateContexts("com.laoy.streamdock.crimson.global", data.championId ? 1 : 0);
            break;
        case 'AUTO_PICK_STATE':
            updateContexts("com.laoy.streamdock.crimson.autopick", data.championId ? 1 : 0);
            updateContexts("com.laoy.streamdock.crimson.global", data.championId ? 1 : 0);
            break;
    }
};


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
    if (action === "com.laoy.streamdock.crimson.display") {
        let title = "";
        let value = "";
        let image = "static/icon/stats.png";

        if (gameState.phase === "ChampSelect" && gameState.champion > 0) {
            title = gameState.championName || "PICK";
            value = "ID: " + gameState.champion;
        } else {
            title = gameState.rank.tier !== "UNRANKED" ? gameState.rank.tier : "RANK";
            value = gameState.rank.tier !== "UNRANKED" ? `${gameState.rank.division} ${gameState.rank.lp} LP` : "UNRANKED";
        }
        
        if (controller === "Information") {
            ui.setPayload(context, title, value, image);
        } else {
            ui.setTitle(context, title + "\n" + value);
        }
    }
    
    if (action === "com.laoy.streamdock.crimson.dodge") {
        const visible = gameState.phase === "ChampSelect" || gameState.phase === "Lobby";
        ui.setTitle(context, visible ? "DODGE" : "");
    }
}


// ==========================================
// 3. ELGATO PLUGIN REGISTRATION
// ==========================================
window.connectElgatoStreamDeckSocket = function(inPort, inPluginUUID, inRegisterEvent, inInfo) {
    const streamDeckSocket = new WebSocket("ws://127.0.0.1:" + inPort);
    ui.setSocket(streamDeckSocket); // Bind socket to UI layer

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
            api.send({ type: 'TOGGLE_AUTO_BAN', championId: 0 }); // Global behavior handled by Rust dispatcher
            break;
        case "com.laoy.streamdock.crimson.autopick":
            api.send({ type: 'TOGGLE_AUTO_PICK', championId: 0 }); // Global behavior handled by Rust dispatcher
            break;
        case "com.laoy.streamdock.crimson.global":
            api.send({ type: 'TOGGLE_GLOBAL_AUTOMATION' });
            break;
        case "com.laoy.streamdock.crimson.inject":
            api.send({ 
                type: 'INJECT_BUILD', 
                championId: gameState.champion,
                championName: gameState.championName
            });
            break;
    }
}
