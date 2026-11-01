package me.sjun.uim

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.ScrollView
import android.widget.TextView

/** The launcher screen: turn the keyboard on, switch to it, choose the bottom row (SPEC §1.2), try it. */
class SetupActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val prefs = getSharedPreferences(UimInputService.PREFS, MODE_PRIVATE)
        val d = resources.displayMetrics.density
        val pad = (20 * d).toInt()
        val col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(pad, pad, pad, pad) }
        fun text(s: String, size: Float = 15f) = TextView(this).apply { this.text = s; textSize = size; setPadding(0, pad / 2, 0, pad / 4) }

        col.addView(text("uim", 26f))
        col.addView(text("English, Korean and Japanese in one keyboard."))
        col.addView(Button(this).apply {
            text = "1. Turn on uim"
            setOnClickListener { startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)) }
        })
        col.addView(Button(this).apply {
            text = "2. Switch to uim"
            setOnClickListener { (getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager).showInputMethodPicker() }
        })

        col.addView(text("Bottom row", 18f))
        val names = listOf(
            "Idempotent, KO priority (default)  S E A a ␣ 한 ひ カ ↵",
            "Idempotent, JA priority  S E A a ␣ ひ カ 한 ↵",
            "Cycle, A  한A1 あA1 A ␣ ↵",
            "Cycle, a  한A1 あA1 a ␣ ↵",
            "Cycle, E  한A1 あA1 E ␣ ↵",
            "E-shift, KO priority  S E⇧ , ␣ 한 ひ カ ↵",
            "E-shift, JA priority  S E⇧ , ␣ ひ カ 한 ↵",
        )
        val group = RadioGroup(this)
        names.forEachIndexed { i, n -> group.addView(RadioButton(this).apply { id = 1000 + i; text = n }) }
        group.check(1000 + prefs.getInt(UimInputService.BOTTOM_KEY, 0))
        group.setOnCheckedChangeListener { _, id -> prefs.edit().putInt(UimInputService.BOTTOM_KEY, id - 1000).apply() }
        col.addView(group)

        col.addView(text("Try it", 18f))
        col.addView(EditText(this).apply { hint = "Type here"; minLines = 3 })
        setContentView(ScrollView(this).apply { addView(col) })
    }
}
