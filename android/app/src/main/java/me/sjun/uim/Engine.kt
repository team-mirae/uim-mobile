package me.sjun.uim

/** The engine's C ABI (engine/include/uim.h), through JNI. One engine per keyboard. */
class Engine {
    private var h = nativeNew()

    class Output(val commit: String, val preedit: String, val action: Int, val arg: Int)

    /** One key of the layout, as the engine describes it (SPEC: the shell draws what it's given). */
    class Key(
        val kind: String, val width: Float, val label: String, val corner: String,
        val legends: List<String>, val flags: Int,
    ) {
        fun has(flag: Int) = flags and flag != 0
        val flicks get() = legends.any { it.isNotEmpty() } && !has(CARET)
    }

    var rows: List<List<Key>> = emptyList()
        private set
    var candidates: List<String> = emptyList()
        private set

    init { reload() }

    fun close() { if (h != 0L) nativeFree(h); h = 0 }

    fun setWide(wide: Boolean) { nativeSetWide(h, wide); reload() }
    fun setBottom(b: Int) { nativeSetBottom(h, b); reload() }
    fun loadWords(data: ByteArray) = nativeLoadWords(h, data)
    fun loadBigrams(data: ByteArray) = nativeLoadBigrams(h, data)
    fun loadAcronyms(data: ByteArray) = nativeLoadAcronyms(h, data)

    /** Gives the engine JA's converter, parsed by [Kanji.parse]; the engine keeps its own reference. */
    fun setKanji(kanji: Kanji) { nativeSetKanji(h, kanji.handle); reload() }

    /** JA's kana-kanji conversion data (engine/data/kanji.bin), parsed once. Safe off the UI thread. */
    class Kanji private constructor(internal val handle: Long) {
        fun close() { nativeKanjiFree(handle) }

        companion object {
            /** Null if the data doesn't parse. */
            fun parse(data: ByteArray): Kanji? = nativeKanjiNew(data).takeIf { it != 0L }?.let(::Kanji)
        }
    }

    fun press(row: Int, col: Int, dir: Int): Output { nativePress(h, row, col, dir); return out() }
    fun hold(on: Boolean): Output { nativeHold(h, on); return out() }
    fun drag(steps: Int): Output { nativeDrag(h, steps); return out() }
    fun candidate(i: Int): Output { nativeCandidate(h, i); return out() }
    fun flush(): Output { nativeFlush(h); return out() }

    private fun out(): Output {
        reload()
        return Output(str(nativeCommit(h)), str(nativePreedit(h)), nativeAction(h), nativeActionArg(h))
    }

    private fun reload() {
        rows = str(nativeLayout(h)).split('\n').map { row ->
            row.split('\u001E').map { k ->
                val f = k.split('\u001F')
                Key(f[0], f[1].toFloat(), f[2], f[3], f.subList(4, 8), f[8].toInt())
            }
        }
        candidates = (0 until nativeCandidateCount(h)).map { str(nativeCandidateAt(h, it)) }
    }

    private fun str(b: ByteArray) = String(b, Charsets.UTF_8)

    companion object {
        init { System.loadLibrary("uim") }

        const val TAP = 0; const val N = 1; const val E = 2; const val S = 3; const val W = 4

        const val ACTION_NONE = 0; const val ACTION_DELETE = 1; const val ACTION_ENTER = 2
        const val ACTION_COPY = 3; const val ACTION_PASTE = 4; const val ACTION_CURSOR = 5

        // Key flags (engine/src/layout.rs).
        const val ON = 1; const val LOCK = 2; const val SMALL = 4; const val CARET = 8; const val OTHER = 16
        const val ICON_COPY = 32; const val ICON_PASTE = 64; const val ICON_BS = 128; const val ICON_ENTER = 256
        const val ICON_CAPS = 512; const val ICON_SHIFT = 1024; const val REPEAT = 2048; const val HOLD = 4096

        @JvmStatic private external fun nativeNew(): Long
        @JvmStatic private external fun nativeFree(h: Long)
        @JvmStatic private external fun nativeSetWide(h: Long, wide: Boolean)
        @JvmStatic private external fun nativeSetBottom(h: Long, b: Int)
        @JvmStatic private external fun nativeLoadWords(h: Long, d: ByteArray)
        @JvmStatic private external fun nativeLoadBigrams(h: Long, d: ByteArray)
        @JvmStatic private external fun nativeLoadAcronyms(h: Long, d: ByteArray)
        @JvmStatic private external fun nativeKanjiNew(d: ByteArray): Long
        @JvmStatic private external fun nativeKanjiFree(k: Long)
        @JvmStatic private external fun nativeSetKanji(h: Long, k: Long)
        @JvmStatic private external fun nativePress(h: Long, row: Int, col: Int, dir: Int)
        @JvmStatic private external fun nativeHold(h: Long, on: Boolean)
        @JvmStatic private external fun nativeDrag(h: Long, steps: Int)
        @JvmStatic private external fun nativeCandidate(h: Long, i: Int)
        @JvmStatic private external fun nativeFlush(h: Long)
        @JvmStatic private external fun nativeCommit(h: Long): ByteArray
        @JvmStatic private external fun nativePreedit(h: Long): ByteArray
        @JvmStatic private external fun nativeLayout(h: Long): ByteArray
        @JvmStatic private external fun nativeAction(h: Long): Int
        @JvmStatic private external fun nativeActionArg(h: Long): Int
        @JvmStatic private external fun nativeCandidateCount(h: Long): Int
        @JvmStatic private external fun nativeCandidateAt(h: Long, i: Int): ByteArray
    }
}
