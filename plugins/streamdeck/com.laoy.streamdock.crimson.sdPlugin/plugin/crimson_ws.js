class CrimsonAPI {
    constructor() {
        this.ws = new WebSocket('ws://localhost:40509');
        this.onMessage = null;

        this.ws.onmessage = (event) => {
            try {
                if (this.onMessage) this.onMessage(JSON.parse(event.data));
            } catch (e) {
                console.error("Crimson WS Payload Parsing Error", e);
            }
        };
    }

    send(command) {
        if (this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(command));
        } else {
            console.warn("Crimson Backend disconnected, command dropped:", command);
        }
    }
}
const api = new CrimsonAPI();
