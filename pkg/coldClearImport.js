// coldClearWrapper.js

// Import the generated JS file from wasm-pack.
// This path assumes the web page is at the root and a `pkg` directory exists at the root.
import init, { WasmBot } from './pkg/cold_clear_2.js';

// We will expose all APIs on this object
const ColdClear = {
    _bot: null,
    _initialized: false,

    /**
     * Initializes the Wasm module and creates a Bot instance.
     * This function must be called before any other API.
     * @param {object} config - The configuration object for the bot. Pass an empty object {} to use defaults.
     * @returns {Promise<void>}
     */
    async initialize(config = {}) {
        if (this._initialized) {
            console.log("Cold Clear has already been initialized.");
            return;
        }

        try {
            // Initialize the Wasm module.
            // wasm-pack's generated JS module will handle loading the .wasm file.
            await init();

            // Create a WasmBot instance
            this._bot = new WasmBot(config);
            this._initialized = true;
            console.log("Cold Clear Wasm bot initialized successfully.");

        } catch (error) {
            console.error("Failed to initialize Cold Clear Wasm bot:", error);
            throw error; // Re-throw the error so the application can handle it
        }
    },

    /**
     * Checks if the bot is initialized.
     * @private
     */
    _checkInitialized() {
        if (!this._initialized || !this._bot) {
            throw new Error("Cold Clear is not initialized. Please call ColdClear.initialize() first.");
        }
    },

    /**
     * Starts a new game round.
     * @param {object} startInfo - The initial state of the game.
     */
    start(startInfo) {
        this._checkInitialized();
        try {
            this._bot.start(startInfo);
        } catch (error) {
            console.error("Error in ColdClear.start():", error);
        }
    },

    /**
     * Notifies the bot of a new piece.
     * @param {string} piece - The name of the piece (e.g., "T", "L", "I").
     */
    newPiece(piece) {
        this._checkInitialized();
        try {
            this._bot.new_piece(piece);
        } catch (error) {
            console.error("Error in ColdClear.newPiece():", error);
        }
    },

    /**
     * Notifies the bot that a move has been played.
     * @param {object} move - Details of the placed piece (Placement object).
     */
    play(move) {
        this._checkInitialized();
        try {
            this._bot.play(move);
        } catch (error) {
            console.error("Error in ColdClear.play():", error);
        }
    },

    /**
     * Requests a move suggestion from the bot.
     * @returns {object | null} - An object containing the suggested move, or null.
     */
    suggest() {
        this._checkInitialized();
        try {
            return this._bot.suggest();
        } catch (error) {
            console.error("Error in ColdClear.suggest():", error);
            return null;
        }
    },

    /**
     * Stops the bot.
     */
    stop() {
        this._checkInitialized();
        this._bot.stop();
    },

    /**
     * Sets the search depth (number of iterations) for the suggest method.
     * @param {number} depth - The number of search iterations.
     */
    setSearchDepth(depth) {
        this._checkInitialized();
        try {
            this._bot.set_search_depth(depth);
        } catch (error) {
            console.error("Error in ColdClear.setSearchDepth():", error);
        }
    }
};

// You can export ColdClear or set it as a global variable
// export default ColdClear;
window.ColdClear = ColdClear;
