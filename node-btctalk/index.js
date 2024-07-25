const axios = require('axios');
const cheerio = require('cheerio');
const fs = require('fs');
const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');

// Lista de proxies no formato [ip, port, username, password]
const proxies = [
    ['', '', '', '']
];

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function scrapeUser(id, proxyConfig) {
    const url = `https://bitcointalk.org/index.php?action=profile;u=${id}`;
    try {
        let options = {};
        if (proxyConfig[0]) {  // Se o proxy não for vazio
            options = {
                proxy: {
                    host: proxyConfig[0],
                    port: parseInt(proxyConfig[1]),
                    protocol: 'http',
                    auth: {
                        username: proxyConfig[2],
                        password: proxyConfig[3]
                    }
                }
            };
        }
        const { data } = await axios.get(url, options);
        const $ = cheerio.load(data);
        const name = $('.windowbg').find('td').eq(1).text().trim();
        if (name) {
            console.log(`ID: ${id}, Name: ${name}`);
            fs.appendFileSync('usernames.txt', `${name}\n`, 'utf8');
        }
    } catch (error) {
        console.error(`Error fetching user ID: ${id}`, error.message);
        fs.appendFileSync('errors.txt', `ID: ${id}, Error: ${error.message}\n`, 'utf8');
    }
}

async function scrapeUsers(startId, endId, proxyConfig) {
    for (let id = startId; id <= endId; id++) {
        await scrapeUser(id, proxyConfig);
        await sleep(1000); // Pausa de 1 segundo entre cada requisição
    }
}

if (isMainThread) {
    const startId = 1;
    const endId = 4500000;
    const numWorkers = proxies.length;
    const range = Math.ceil((endId - startId + 1) / numWorkers);

    for (let i = 0; i < numWorkers; i++) {
        const workerStartId = startId + i * range;
        const workerEndId = Math.min(workerStartId + range - 1, endId);
        const proxyConfig = proxies[i];

        const worker = new Worker(__filename, {
            workerData: { workerStartId, workerEndId, proxyConfig }
        });

        worker.on('message', (message) => console.log(`Worker ${i} finished`));
        worker.on('error', (error) => console.error(`Worker ${i} error: ${error}`));
        worker.on('exit', (code) => {
            if (code !== 0)
                console.error(`Worker ${i} stopped with exit code ${code}`);
        });
    }
} else {
    const { workerStartId, workerEndId, proxyConfig } = workerData;
    scrapeUsers(workerStartId, workerEndId, proxyConfig).then(() => {
        parentPort.postMessage('done');
    });
}
