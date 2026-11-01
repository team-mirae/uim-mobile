package me.sjun.uim

import android.content.res.Configuration
import android.inputmethodservice.InputMethodService
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo

/**
 * The input method: sends (row, key, direction) to the engine and applies its output. It never
 * decides what a key types (SPEC §0 principle 4).
 */
class UimInputService : InputMethodService() {
    private lateinit var engine: Engine
    private var view: KeyboardView? = null
    private var composing = false
    private val handler = Handler(Looper.getMainLooper())
    private var destroyed = false
    private val prefs by lazy { getSharedPreferences(PREFS, MODE_PRIVATE) }

    override fun onCreate() {
        super.onCreate()
        engine = Engine()
        // EN predictive's word list (SPEC §3.3) and JA's kana-kanji converter (SPEC §6), read and
        // parsed off the main thread.
        Thread {
            val words = assets.open("en-words.txt").use { it.readBytes() }
            val bigrams = assets.open("en-bigrams.txt").use { it.readBytes() }
            val acronyms = assets.open("en-acronyms.txt").use { it.readBytes() }
            handler.post {
                if (!destroyed) {
                    engine.loadWords(words)
                    engine.loadBigrams(bigrams)
                    engine.loadAcronyms(acronyms)
                }
            }
            val kanji = Engine.Kanji.parse(assets.open("kanji.bin").use { it.readBytes() })
            handler.post {
                if (!destroyed && kanji != null) {
                    engine.setKanji(kanji)
                    view?.refresh()
                }
                kanji?.close()
            }
        }.start()
    }

    override fun onDestroy() {
        destroyed = true
        engine.close()
        super.onDestroy()
    }

    override fun onCreateInputView(): View =
        KeyboardView(this, engine, ::press, ::candidate, ::drag, ::hold).also { view = it }

    override fun onStartInputView(info: EditorInfo?, restarting: Boolean) {
        super.onStartInputView(info, restarting)
        applySettings()
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        // Folding or unfolding: commit first, so the layout never changes mid-syllable (SPEC §2).
        commitComposing()
        applySettings()
    }

    /** Screen class (SPEC §2): wide from 600 dp, and the bottom-row setting (SPEC §1.2). */
    private fun applySettings() {
        engine.setWide(resources.configuration.screenWidthDp >= WIDE_DP)
        engine.setBottom(prefs.getInt(BOTTOM_KEY, 0))
        view?.refresh()
    }

    override fun onStartInput(attribute: EditorInfo?, restarting: Boolean) {
        super.onStartInput(attribute, restarting)
        engine.flush()
        composing = false
        view?.refresh()
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        commitComposing()
        super.onFinishInputView(finishingInput)
    }

    override fun onFinishInput() {
        commitComposing()
        super.onFinishInput()
    }

    override fun onUpdateSelection(
        oldSelStart: Int, oldSelEnd: Int, newSelStart: Int, newSelEnd: Int, candidatesStart: Int, candidatesEnd: Int,
    ) {
        super.onUpdateSelection(oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd)
        // The user moved the cursor away from the composing text: commit it as shown.
        if (composing && (newSelStart != newSelEnd || newSelEnd != candidatesEnd)) commitComposing()
    }

    private fun commitComposing() {
        engine.flush()
        if (composing) currentInputConnection?.finishComposingText()
        composing = false
        view?.refresh()
    }

    private fun press(row: Int, col: Int, dir: Int) = apply(engine.press(row, col, dir))
    private fun candidate(i: Int) = apply(engine.candidate(i))
    private fun drag(steps: Int) = apply(engine.drag(steps))
    private fun hold(on: Boolean) = apply(engine.hold(on))

    private fun apply(out: Engine.Output) {
        val ic = currentInputConnection
        if (ic != null) {
            ic.beginBatchEdit()
            if (out.commit.isNotEmpty()) {
                ic.commitText(out.commit, 1) // replaces the composing text
                composing = false
            }
            if (out.preedit.isNotEmpty()) {
                ic.setComposingText(out.preedit, 1)
                composing = true
            } else if (composing) {
                ic.setComposingText("", 1)
                ic.finishComposingText()
                composing = false
            }
            when (out.action) {
                Engine.ACTION_DELETE -> delete()
                Engine.ACTION_ENTER -> enter()
                Engine.ACTION_COPY -> ic.performContextMenuAction(android.R.id.copy)
                Engine.ACTION_PASTE -> ic.performContextMenuAction(android.R.id.paste)
                Engine.ACTION_CURSOR -> {
                    val code = if (out.arg < 0) KeyEvent.KEYCODE_DPAD_LEFT else KeyEvent.KEYCODE_DPAD_RIGHT
                    repeat(kotlin.math.abs(out.arg)) { sendDownUpKeyEvents(code) }
                }
            }
            ic.endBatchEdit()
        }
        view?.refresh()
    }

    /** Backspace with nothing composing: delete the selection, or the character before the cursor. */
    private fun delete() {
        val ic = currentInputConnection ?: return
        val selected = ic.getSelectedText(0)
        if (!selected.isNullOrEmpty()) ic.commitText("", 1) else sendDownUpKeyEvents(KeyEvent.KEYCODE_DEL)
    }

    /** Enter: the field's action (send, search, go) on a single-line field that has one, else a new line. */
    private fun enter() {
        val info = currentInputEditorInfo
        val action = info.imeOptions and EditorInfo.IME_MASK_ACTION
        val multiLine = info.inputType and InputType.TYPE_TEXT_FLAG_MULTI_LINE != 0
        val noAction = info.imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION != 0
        if (!multiLine && !noAction && action != EditorInfo.IME_ACTION_NONE && action != EditorInfo.IME_ACTION_UNSPECIFIED) {
            currentInputConnection?.performEditorAction(action)
        } else {
            sendKeyChar('\n')
        }
    }

    companion object {
        const val PREFS = "uim"
        const val BOTTOM_KEY = "bottom-row"
        const val WIDE_DP = 600
    }
}
