// coldClearWrapper.js

// 引入從 wasm-pack 產生的 JS 檔案
import init, { WasmBot } from './pkg/cold_clear_2.js';

// 我們將在這個物件中公開所有 API
const ColdClear = {
    _bot: null,
    _initialized: false,

    /**
     * 初始化 Wasm 模組並建立 Bot 實例。
     * 必須在使用任何其他 API 之前呼叫此函式。
     * @param {object} config - Bot 的設定物件。傳入空物件 {} 可使用預設值。
     * @returns {Promise<void>}
     */
    async initialize(config = {}) {
        if (this._initialized) {
            console.log("Cold Clear has already been initialized.");
            return;
        }

        try {
            // 初始化 Wasm 模組
            // 假設 wasm 檔案相對於根目錄的 pkg/ 資料夾
            await init('./pkg/cold_clear_2_bg.wasm');

            // 建立 WasmBot 實例
            this._bot = new WasmBot(config);
            this._initialized = true;
            console.log("Cold Clear Wasm bot initialized successfully.");

        } catch (error) {
            console.error("Failed to initialize Cold Clear Wasm bot:", error);
            throw error; // 拋出錯誤以便應用程式可以處理
        }
    },

    /**
     * 檢查 Bot 是否已初始化。
     * @private
     */
    _checkInitialized() {
        if (!this._initialized || !this._bot) {
            throw new Error("Cold Clear is not initialized. Please call ColdClear.initialize() first.");
        }
    },

    /**
     * 啟動一個新的遊戲回合。
     * @param {object} startInfo - 遊戲的初始狀態。
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
     * 通知 Bot 一個新的方塊。
     * @param {string} piece - 方塊的名稱 (e.g., "T", "L", "I").
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
     * 通知 Bot 一個移動已經被執行。
     * @param {object} move - 已放置方塊的詳細資訊 (Placement object).
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
     * 向 Bot 請求移動建議。
     * @returns {object | null} - 包含建議移動的物件，或 null。
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
     * 停止 Bot。
     */
    stop() {
        this._checkInitialized();
        this._bot.stop();
    }
};

// 您可以將 ColdClear 匯出，或將其設為全域變數
// export default ColdClear;
window.ColdClear = ColdClear;
