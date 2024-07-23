const axios = require('axios');
const cheerio = require('cheerio');
const fs = require('fs');

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function scrapeUser(id) {
    const url = `https://bitcointalk.org/index.php?action=profile;u=${id}`;
    try {
        const { data } = await axios.get(url);
        const $ = cheerio.load(data);
        const name = $('.windowbg').find('td').eq(1).text().trim();
        if (name) {
            console.log(`ID: ${id}, Name: ${name}`);
            fs.appendFileSync('usernames.txt', `${name}\n`, 'utf8');
        }
    } catch (error) {
        console.error(`Error fetching user ID: ${id}`, error.message);
    }
}

async function scrapeUsers(startId, endId) {
    for (let id = startId; id <= endId; id++) {
        await scrapeUser(id);
        await sleep(1000); // Pausa de 1 segundo entre cada requisição
    }
}

const startId = 1;
const endId = 1000000;

scrapeUsers(startId, endId);
