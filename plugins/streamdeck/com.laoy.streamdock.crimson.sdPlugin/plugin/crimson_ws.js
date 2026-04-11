class CrimsonAPI {
    constructor() {
        this.url = 'ws://127.0.0.1:40509';
        this.ws = null;
        this.onMessage = null;
        this.onStatusChange = null;
        this.reconnectInterval = 1000;
        this.maxReconnectInterval = 30000;
        this.isConnected = false;
        this.attempts = 0;
        this.connect();
    }

    connect() {
        this.attempts++;
        console.log(`CrimsonAPI: Connecting to ${this.url} (Attempt ${this.attempts})...`);
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
            console.log("CrimsonAPI: Connected to Backend.");
            this.reconnectInterval = 1000; 
            this.isConnected = true;
            if (this.onStatusChange) this.onStatusChange(true);
        };

        this.ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                if (this.onMessage) this.onMessage(data);
            } catch (e) {
                console.error("Crimson WS Payload Parsing Error", e);
            }
        };

        this.ws.onclose = () => {
            console.warn(`CrimsonAPI: Connection lost. Reconnecting in ${this.reconnectInterval / 1000}s...`);
            this.isConnected = false;
            if (this.onStatusChange) this.onStatusChange(false);
            setTimeout(() => {
                this.reconnectInterval = Math.min(this.reconnectInterval * 2, this.maxReconnectInterval);
                this.connect();
            }, this.reconnectInterval);
        };

        this.ws.onerror = () => {
            this.ws.close();
        };
    }

    send(command) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(command));
        } else {
            console.warn("Crimson Backend disconnected, command dropped:", command);
        }
    }
}
const api = new CrimsonAPI();

