let streamDeckSocket = null;
let globalPluginUUID = "";
let actionContexts = {
    "com.laoy.streamdock.discord.togglemute": [],
    "com.laoy.streamdock.discord.toggledeafen": [],
    "com.laoy.streamdock.discord.joinvoice": [],
    "com.laoy.streamdock.discord.togglecamera": []
};

// Discord Integration State
let currentVoiceSettings = { mute: false, deaf: false };
let currentVideoState = { cameraOn: false };

// ==========================================
// CRIMSON BACKEND CONNECT
// ==========================================
const crimsonAPI = {
    ws: null,
    queue: [],
    onOpen: null,
    connect() {
        this.ws = new WebSocket("ws://127.0.0.1:40510");
        this.ws.onopen = () => {
            console.log("[Discord] Connected to Crimson Backend on 40510.");
            if (this.onOpen) {
                this.onOpen();
            }
            while (this.queue.length > 0) {
                this.ws.send(JSON.stringify(this.queue.shift()));
            }
        };
        this.ws.onmessage = (evt) => {
            try {
                const data = JSON.parse(evt.data);
                if (data.event === "setState") {
                    updateActionState(data.action, data.payload.state);
                }
                if (data.type === "DISCORD_STATE" && data.data) {
                    const state = data.data;
                    currentVoiceSettings.mute = state.is_muted;
                    currentVoiceSettings.deaf = state.is_deaf;
                    currentVideoState.cameraOn = state.is_camera_on;
                    
                    updateActionState("com.laoy.streamdock.discord.togglemute", state.is_muted ? 1 : 0);
                    updateActionState("com.laoy.streamdock.discord.toggledeafen", state.is_deaf ? 1 : 0);
                    updateActionState("com.laoy.streamdock.discord.togglecamera", state.is_camera_on ? 1 : 0);
                }
            } catch (e) {
                console.error("[Discord] Error parsing Crimson message", e);
            }
        };
        this.ws.onclose = () => {
            console.warn("[Discord] Crimson connection closed. Reconnecting in 2s...");
            setTimeout(() => this.connect(), 2000);
            
            setTimeout(() => {
                if (!streamDeckSocket || streamDeckSocket.readyState === WebSocket.CLOSED) {
                    if (window.connectHw) {
                        window.connectHw();
                    }
                }
            }, 1000);
        };
        this.ws.onerror = (err) => {
            console.error("[Discord] Crimson connection error", err);
            this.ws.close();
        };
    },
    send(data) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(data));
        } else {
            this.queue.push(data);
        }
    }
};
crimsonAPI.connect();

/**
 * Entry point for StreamDock Plugin
 */
function connectElgatoStreamDeckSocket(inPort, inPluginUUID, inRegisterEvent, inInfo) {
    globalPluginUUID = inPluginUUID;

    const register = () => {
        crimsonAPI.send({
            type: 'REGISTER_STREAMDOCK',
            port: inPort,
            uuid: inPluginUUID,
            register_event: inRegisterEvent
        });
    };

    crimsonAPI.onOpen = register;
    if (crimsonAPI.ws && crimsonAPI.ws.readyState === WebSocket.OPEN) {
        register();
    }

    const connectHw = () => {
        if (crimsonAPI.ws && crimsonAPI.ws.readyState === WebSocket.OPEN) {
            console.log("[Discord] Crimson Server is active. Skipping direct hardware connection.");
            return;
        }

        streamDeckSocket = new WebSocket("ws://127.0.0.1:" + inPort);

        streamDeckSocket.onopen = function () {
            const json = { "event": inRegisterEvent, "uuid": inPluginUUID };
            streamDeckSocket.send(JSON.stringify(json));
        };

        streamDeckSocket.onclose = function () {
            console.warn("[Discord] Hardware socket closed.");
            setTimeout(() => {
                if (!crimsonAPI.ws || crimsonAPI.ws.readyState !== WebSocket.OPEN) {
                    connectHw();
                }
            }, 3000);
        };

        streamDeckSocket.onerror = function () {
            streamDeckSocket.close();
        };

        streamDeckSocket.onmessage = function (evt) {
            const jsonObj = JSON.parse(evt.data);
            const event = jsonObj['event'];
            const action = jsonObj['action'];
            const context = jsonObj['context'];

            if (event === "willAppear") {
                if (actionContexts[action] && !actionContexts[action].includes(context)) {
                    actionContexts[action].push(context);
                }
            }

            if (event === "willDisappear") {
                if (actionContexts[action]) {
                    actionContexts[action] = actionContexts[action].filter(c => c !== context);
                }
            }

            if (event === "keyDown") {
                handleKeyDown(action, context);
            }
        };
    };

    window.connectHw = connectHw;
    connectHw();
}

async function handleKeyDown(action, context) {
    let execAction = null;
    let execParams = {};

    switch (action) {
        case "com.laoy.streamdock.discord.togglemute":
            execAction = 'toggleMute';
            break;
        case "com.laoy.streamdock.discord.toggledeafen":
            execAction = 'toggleDeafen';
            break;
        case "com.laoy.streamdock.discord.joinvoice":
            execAction = 'joinVoiceChannel';
            execParams = { channelId: "YOUR_CHANNEL_ID_HERE" }; // User should configure this in PI
            break;
        case "com.laoy.streamdock.discord.togglecamera":
            execAction = 'toggleCamera';
            break;
    }

    if (execAction) {
        console.log(`[Discord] Executing ${execAction}...`);
        // Forward to Crimson backend
        crimsonAPI.send({
            type: "DISCORD_COMMAND",
            endpoint: execAction,
            payload: execParams
        });
    }
}

function updateActionState(action, state) {
    const contexts = actionContexts[action] || [];
    contexts.forEach(context => {
        const json = { "event": "setState", "context": context, "payload": { "state": state } };
        if (streamDeckSocket && streamDeckSocket.readyState === WebSocket.OPEN) {
            streamDeckSocket.send(JSON.stringify(json));
        } else if (crimsonAPI.ws && crimsonAPI.ws.readyState === WebSocket.OPEN) {
            crimsonAPI.send({
                type: "FORWARD_TO_STREAMDOCK",
                payload: json
            });
        }
    });
}

window.connectElgatoStreamDeckSocket = connectElgatoStreamDeckSocket;
