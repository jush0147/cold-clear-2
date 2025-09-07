async function main() {
    const { ColdClearBot } = await import('./pkg/cold_clear_2.js');

    let bot = null;

    document.getElementById('start').addEventListener('click', () => {
        if (bot) {
            console.log('Bot already started');
            return;
        }
        console.log('Starting bot...');
        const config = document.getElementById('config').value;
        bot = new ColdClearBot((message) => {
            console.log('Received message from bot:', message);
        }, config);
        console.log('Bot started.');
    });

    document.getElementById('send').addEventListener('click', () => {
        if (!bot) {
            console.error('Bot not started yet');
            return;
        }
        const message = document.getElementById('message').value;
        console.log('Sending message to bot:', message);
        bot.send_message(message);
    });
}

main().catch(console.error);
