const { execSync } = require('child_process');
const https = require('https');

function getLCUInfo() {
    try {
        const cmd = `powershell -NoProfile -NonInteractive -Command "(Get-Process -Name LeagueClientUx -ErrorAction SilentlyContinue | Select-Object -First 1).CommandLine"`;
        const output = execSync(cmd, { encoding: 'utf-8' });
        const portMatch = output.match(/--app-port=(\d+)/);
        const passMatch = output.match(/--remoting-auth-token=([^"\s]+)/);
        if (portMatch && passMatch) {
            return { port: portMatch[1], token: passMatch[1] };
        }
    } catch {}
    return null;
}

const info = getLCUInfo();
if (!info) {
    console.log("No LCU found");
    process.exit(1);
}

const auth = Buffer.from(`riot:${info.token}`).toString('base64');
const agent = new https.Agent({ rejectUnauthorized: false });

function lcuFetch(endpoint) {
    return new Promise((resolve) => {
        https.get(`https://127.0.0.1:${info.port}${endpoint}`, {
            headers: { Authorization: `Basic ${auth}` },
            agent
        }, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => resolve(JSON.parse(data)));
        }).on('error', () => resolve(null));
    });
}

async function run() {
    const sum = await lcuFetch('/lol-summoner/v1/current-summoner');
    console.log("puuid:", sum.puuid);
    console.log("sumId:", sum.summonerId);

    const rs1 = await lcuFetch('/lol-ranked/v1/current-ranked-stats');
    console.log("\n/lol-ranked/v1/current-ranked-stats");
    console.log(JSON.stringify(rs1.queues || rs1, null, 2));

    const rs2 = await lcuFetch(`/lol-ranked/v1/ranked-stats/${sum.puuid}`);
    console.log("\n/lol-ranked/v1/ranked-stats/puuid");
    console.log(JSON.stringify(rs2.queues || rs2, null, 2));
}
run();
