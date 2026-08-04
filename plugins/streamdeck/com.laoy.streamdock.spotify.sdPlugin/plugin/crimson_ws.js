/** Read %APPDATA%\com.laoy.crimsons\auth.token (same path as the local server). */
function crimsonAuthToken() {
    try {
        var fs = require('fs');
        var path = require('path');
        var tokenPath = path.join(process.env.APPDATA || '', 'com.laoy.crimsons', 'auth.token');
        return (fs.readFileSync(tokenPath, 'utf8') || '').trim();
    } catch (e) {
        try {
            var shell = new ActiveXObject('WScript.Shell');
            var fso = new ActiveXObject('Scripting.FileSystemObject');
            var p = shell.ExpandEnvironmentStrings('%APPDATA%\\com.laoy.crimsons\\auth.token');
            if (!fso.FileExists(p)) return '';
            var f = fso.OpenTextFile(p, 1);
            var t = f.ReadAll();
            f.Close();
            return (t || '').trim();
        } catch (e2) {
            return '';
        }
    }
}

function crimsonWsUrl(port) {
    port = port || 40510;
    var token = crimsonAuthToken();
    var base = 'ws://127.0.0.1:' + port;
    return token ? (base + '/?token=' + encodeURIComponent(token)) : base;
}

class CrimsonAPI {
    constructor(port = 40510) {
        this.port = port;
        this.url = crimsonWsUrl(port);
        this.ws = null;
        this.queue = [];
        this.attempts = 0;
        this.connect();
    }

    connect() {
        this.attempts++;
        this.url = crimsonWsUrl(this.port);
        this.ws = new WebSocket(this.url);
        
        this.ws.onopen = () => {
            console.log("CrimsonAPI: Connected to Backend.");
            this.attempts = 0;
            // Flush queue
            while (this.queue.length > 0) {
                const msg = this.queue.shift();
                this.ws.send(JSON.stringify(msg));
            }
        };

        this.ws.onmessage = (evt) => {
            try {
                const data = JSON.parse(evt.data);
                if (window.handleCrimsonMessage) {
                    window.handleCrimsonMessage(data);
                }
            } catch (e) {
                console.error("CrimsonAPI: Error parsing message", e);
            }
        };

        this.ws.onclose = () => {
            console.warn("CrimsonAPI: Connection closed. Reconnecting...");
            setTimeout(() => this.connect(), 2000);
        };

        this.ws.onerror = (err) => {
            console.error("CrimsonAPI: Socket error", err);
        };
    }

    send(data) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(data));
        } else {
            console.log("CrimsonAPI: Socket not open. Queuing message.");
            this.queue.push(data);
        }
    }
}

