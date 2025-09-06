import init, { bot_io } from './cold_clear_2.js';

async function main() {
    // Initialize the wasm module
    await init();

    // After initialization, set up the message handler
    self.onmessage = (e) => {
        try {
            // Call the exported Rust function
            const result = bot_io(e.data);
            // Post the result back to the main thread
            self.postMessage(result);
        } catch (error) {
            // If the Rust function throws an error, post it back
            self.postMessage({ type: 'error', error: error.toString() });
        }
    };

    // Notify the main thread that the worker is ready
    self.postMessage({ type: 'ready' });
}

main().catch(console.error);
