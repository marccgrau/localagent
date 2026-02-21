const { Client, LocalAuth } = require('whatsapp-web.js');
const qrcode = require('qrcode-terminal');
const axios = require('axios');

// Configuration
const BRIDGE_URL = 'http://127.0.0.1:3000/webhook';

// Initialize Client
const client = new Client({
    authStrategy: new LocalAuth(),
    puppeteer: {
        headless: true,
        args: ['--no-sandbox']
    }
});

client.on('qr', (qr) => {
    console.log('QR RECEIVED', qr);
    qrcode.generate(qr, { small: true });
});

client.on('ready', () => {
    console.log('Client is ready!');
});

client.on('message', async msg => {
    // Ignore status updates or empty messages
    if (msg.body === '') return;
    
    console.log(`Received from ${msg.from}: ${msg.body}`);

    try {
        // Forward to Rust Bridge
        const response = await axios.post(BRIDGE_URL, {
            chat_id: msg.from,
            sender_name: msg._data.notifyName || 'Unknown',
            content: msg.body
        });

        // Send Reply
        if (response.data && response.data.reply) {
            console.log(`Replying: ${response.data.reply}`);
            msg.reply(response.data.reply);
        }
    } catch (error) {
        console.error('Error contacting bridge:', error.message);
        if (error.code === 'ECONNREFUSED') {
            msg.reply('[System] LocalGPT bridge is offline.');
        }
    }
});

client.initialize();
