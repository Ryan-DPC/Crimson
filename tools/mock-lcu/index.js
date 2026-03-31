import http from 'http';
import fs from 'fs';

const PORT = 34567;
const PASSWORD = 'mock_password';
const PROTOCOL = 'http';
const LOCKFILE_PATH = '../../apps/desktop/mock_lockfile';

// Write the mock lockfile so Rust can find it
const lockfileContent = `LeagueClient:${process.pid}:${PORT}:${PASSWORD}:${PROTOCOL}`;
fs.writeFileSync(LOCKFILE_PATH, lockfileContent);
console.log(`Mock LCU Lockfile created at: ${LOCKFILE_PATH}`);

const server = http.createServer((req, res) => {
    console.log(`[Mock LCU] Received ${req.method} request to ${req.url}`);
    
    // Check Auth
    const authHeader = req.headers.authorization;
    const expectedAuth = 'Basic ' + Buffer.from(`riot:${PASSWORD}`).toString('base64');
    
    if (authHeader !== expectedAuth) {
        res.writeHead(401);
        res.end(JSON.stringify({ errorCode: "UNAUTHORIZED", message: "Unauthorized access" }));
        return;
    }

    res.setHeader('Content-Type', 'application/json');

    // Handle endpoint matching
    if (req.method === 'POST' && req.url === '/lol-perks/v1/pages') {
        let body = '';
        req.on('data', chunk => body += chunk.toString());
        req.on('end', () => {
            console.log(`[Mock LCU] Injecting runes: ${body}`);
            res.writeHead(200);
            res.end(JSON.stringify({ id: Math.floor(Math.random() * 100), name: JSON.parse(body).name }));
        });
        return;
    }
    
    if (req.method === 'GET' && req.url === '/lol-perks/v1/pages') {
        // Return dummy existing pages
        res.writeHead(200);
        res.end(JSON.stringify([
            { id: 1, name: "LOA: Old Build", current: false },
            { id: 2, name: "LOA: Other Build", current: true }
        ]));
        return;
    }

    if (req.method === 'DELETE' && req.url.startsWith('/lol-perks/v1/pages/')) {
        res.writeHead(204);
        res.end();
        return;
    }

    // Default fallback
    res.writeHead(404);
    res.end(JSON.stringify({ errorCode: "NOT_FOUND", message: "Endpoint not specifically mocked." }));
});

server.listen(PORT, '127.0.0.1', () => {
    console.log(`[Mock LCU] Server listening on http://127.0.0.1:${PORT}`);
    console.log(`Run Crimson with: CRIMSON_MOCK_LCU=true npm run tauri dev`);
});

process.on('SIGINT', () => {
    console.log("\n[Mock LCU] Shutting down, cleaning lockfile...");
    if (fs.existsSync(LOCKFILE_PATH)) {
        fs.unlinkSync(LOCKFILE_PATH);
    }
    process.exit(0);
});
