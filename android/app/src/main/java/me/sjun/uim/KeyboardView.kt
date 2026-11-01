package me.sjun.uim

import android.annotation.SuppressLint
import android.content.Context
import android.content.res.Configuration
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.text.TextUtils
import android.text.TextPaint
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.view.WindowInsets
import kotlin.math.abs
import kotlin.math.hypot

/**
 * Draws the engine's layout (SPEC §1: 13u grid, 6 equal rows) and turns touches into
 * (row, key, direction). `hit` is the only place a touch becomes a key; no dead zones.
 */
@SuppressLint("ViewConstructor")
class KeyboardView(
    context: Context,
    private val engine: Engine,
    private val onKey: (row: Int, col: Int, dir: Int) -> Unit,
    private val onCandidate: (index: Int) -> Unit,
    private val onDrag: (steps: Int) -> Unit,
    private val onHold: (on: Boolean) -> Unit,
) : View(context) {
    private val density = resources.displayMetrics.density
    private val ydpi = resources.displayMetrics.ydpi

    // Flick Korean's colors (light and night).
    private val night = (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
    private val plateColor = if (night) 0xFF1D1A1B.toInt() else 0xFFD8D0C8.toInt()
    private val capColor = if (night) 0xFF3D3839.toInt() else 0xFFFBFAF8.toInt()
    private val modColor = if (night) 0xFF33302D.toInt() else 0xFFE3D9CC.toInt()
    private val edgeColor = if (night) 0xFF1A1718.toInt() else 0xFFC0B7B1.toInt()
    private val onColor = if (night) 0xFFB88A5C.toInt() else 0xFFC79B6C.toInt()
    private val textColor = if (night) 0xFFECE7E3.toInt() else 0xFF433E3F.toInt()
    private val flickColor = if (night) 0xFFD2A676.toInt() else 0xFF8E6E53.toInt()
    private val mutedColor = if (night) 0xFFC0B7B1.toInt() else 0xFF7A7265.toInt()
    private val accentColor = if (night) 0xFF8CC6E0.toInt() else 0xFF2F6F8F.toInt()

    private val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val text = Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }
    private val candText = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }
    private val radius = 8 * density
    private val gap = 3 * density

    /** Row height: 7.5 mm (SPEC §1), in physical units. */
    private val rowHeight = 7.5f / 25.4f * ydpi

    private var rects: List<List<RectF>> = emptyList()

    /** The navigation bar's share of our window (edge-to-edge, Android 15+): keys stay clear of it. */
    private var insetBottom = 0
    private var insetLeft = 0
    private var insetRight = 0

    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        val (l, r, b) = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val i = insets.getInsets(WindowInsets.Type.navigationBars() or WindowInsets.Type.displayCutout())
            Triple(i.left, i.right, i.bottom)
        } else {
            @Suppress("DEPRECATION")
            Triple(insets.systemWindowInsetLeft, insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
        }
        if (l != insetLeft || r != insetRight || b != insetBottom) {
            insetLeft = l; insetRight = r; insetBottom = b
            requestLayout()
            layoutKeys(width)
            invalidate()
        }
        return insets
    }

    private class Touch(val row: Int, val col: Int, val x0: Float, val y0: Float) {
        var dir = Engine.TAP
        var caret = false
        var caretSent = 0
        var candidate = -1
        /** A HOLD key (E/⇧): down; `used` once another key went down meanwhile (then no tap). */
        var hold = false
        var used = false
    }

    private val touches = HashMap<Int, Touch>()
    private val handler = Handler(Looper.getMainLooper())
    private var repeating: Runnable? = null

    fun refresh() {
        layoutKeys(width)
        invalidate()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val w = MeasureSpec.getSize(widthMeasureSpec)
        val h = (6 * rowHeight + gap).toInt() + insetBottom
        setMeasuredDimension(w, h)
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) = layoutKeys(w)

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        requestApplyInsets()
    }

    /** Keys at their place on the 13u grid: x from the running sum of widths, so columns line up. */
    private fun layoutKeys(fullWidth: Int) {
        if (fullWidth == 0) return
        val w = fullWidth - insetLeft - insetRight
        val left = insetLeft.toFloat()
        rects = engine.rows.mapIndexed { r, row ->
            val total = row.sumOf { it.width.toDouble() }.toFloat()
            var cum = 0f
            val top = gap / 2 + r * rowHeight
            row.map { k ->
                val x0 = w * cum / total
                cum += k.width
                val x1 = w * cum / total
                RectF(left + x0 + gap / 2, top + gap / 2, left + x1 - gap / 2, top + rowHeight - gap / 2)
            }
        }
        excludeEdgeGestures(fullWidth)
    }

    /**
     * Gesture navigation treats a swipe in from a screen edge as Back and cancels our touch, so
     * edge keys couldn't flick outwards. Opt the flicking edge keys out (the system honours up to
     * 200 dp per edge).
     */
    private fun excludeEdgeGestures(fullWidth: Int) {
        val zones = ArrayList<android.graphics.Rect>()
        for ((r, row) in rects.withIndex()) {
            val keys = engine.rows.getOrNull(r) ?: continue
            if (keys.none { it.flicks }) continue
            val first = row.firstOrNull() ?: continue
            val last = row.last()
            zones += android.graphics.Rect(0, first.top.toInt(), first.right.toInt(), first.bottom.toInt())
            zones += android.graphics.Rect(last.left.toInt(), last.top.toInt(), fullWidth, last.bottom.toInt())
        }
        systemGestureExclusionRects = zones
    }

    // ---- touch

    /** The key a touch belongs to: the one containing it, else the nearest centre (no dead zones). */
    private fun hit(x: Float, y: Float): Pair<Int, Int>? {
        var best: Pair<Int, Int>? = null
        var bestD = Float.MAX_VALUE
        for ((r, row) in rects.withIndex()) for ((c, rect) in row.withIndex()) {
            val k = engine.rows.getOrNull(r)?.getOrNull(c) ?: continue
            if (k.kind == "rsv") continue
            if (rect.contains(x, y)) return r to c
            val d = hypot(rect.centerX() - x, rect.centerY() - y)
            if (d < bestD) { bestD = d; best = r to c }
        }
        return best
    }

    private fun key(t: Touch) = engine.rows.getOrNull(t.row)?.getOrNull(t.col)
    private fun rect(t: Touch) = rects.getOrNull(t.row)?.getOrNull(t.col)

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(e: MotionEvent): Boolean {
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val i = e.actionIndex
                val x = e.getX(i); val y = e.getY(i)
                val (r, c) = hit(x, y) ?: return true
                val t = Touch(r, c, x, y)
                val k = key(t) ?: return true
                performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                for (other in touches.values) if (other.hold) other.used = true
                if (k.has(Engine.HOLD)) {
                    t.hold = true
                    onHold(true)
                } else if (k.kind == "cand") {
                    t.candidate = candidateAt(x)
                } else if (k.has(Engine.REPEAT)) {
                    onKey(r, c, Engine.TAP)
                    startRepeat(r, c)
                }
                touches[e.getPointerId(i)] = t
                invalidate()
            }
            MotionEvent.ACTION_MOVE -> {
                for (i in 0 until e.pointerCount) {
                    val t = touches[e.getPointerId(i)] ?: continue
                    val k = key(t) ?: continue
                    val rect = rect(t) ?: continue
                    val dx = e.getX(i) - t.x0; val dy = e.getY(i) - t.y0
                    val threshold = rect.width() * FLICK_THRESHOLD
                    if (k.has(Engine.CARET)) {
                        // The caret key: a sideways drag moves the cursor, a step per half threshold.
                        if (!t.caret && abs(dx) >= threshold) t.caret = true
                        if (t.caret) {
                            val steps = ((dx) / (threshold / 2)).toInt()
                            if (steps != t.caretSent) { onDrag(steps - t.caretSent); t.caretSent = steps }
                        }
                    } else if (k.flicks) {
                        t.dir = direction(t, dx, dy, threshold)
                    }
                }
                invalidate()
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val t = touches.remove(e.getPointerId(e.actionIndex)) ?: return true
                val k = key(t)
                when {
                    t.hold -> {
                        onHold(false)
                        if (!t.used) onKey(t.row, t.col, Engine.TAP)
                    }
                    k == null -> {}
                    k.kind == "cand" -> if (t.candidate >= 0) onCandidate(t.candidate)
                    k.has(Engine.REPEAT) -> stopRepeat()
                    t.caret -> {}
                    else -> {
                        val rect = rect(t)
                        val dir = if (k.flicks && rect != null) {
                            direction(t, e.getX(e.actionIndex) - t.x0, e.getY(e.actionIndex) - t.y0, rect.width() * FLICK_THRESHOLD)
                        } else Engine.TAP
                        onKey(t.row, t.col, dir)
                    }
                }
                invalidate()
            }
            MotionEvent.ACTION_CANCEL -> {
                if (touches.values.any { it.hold }) onHold(false)
                touches.clear(); stopRepeat(); invalidate()
            }
        }
        return true
    }

    /** No diagonals: the axis the movement is closest to, once past the threshold (SPEC §4.2). */
    private fun direction(t: Touch, dx: Float, dy: Float, threshold: Float): Int {
        // Near a screen edge the finger hits the frame before the threshold: its reported centre
        // stops well short of the edge. Towards an edge the threshold shrinks to fit the room left.
        val room = maxOf(0f, (if (dx < 0) t.x0 else width - t.x0) - EDGE_DEAD_DP * density)
        val minT = EDGE_MIN_THRESHOLD_DP * density
        val horiz = minOf(threshold, maxOf(minT, room * EDGE_ROOM_SHARE))
        return when {
            abs(dx) > abs(dy) -> when {
                abs(dx) < horiz -> Engine.TAP
                dx > 0 -> Engine.E
                else -> Engine.W
            }
            hypot(dx, dy) < threshold -> Engine.TAP
            dy > 0 -> Engine.S
            else -> Engine.N
        }
    }

    private fun startRepeat(r: Int, c: Int) {
        stopRepeat()
        val run = object : Runnable {
            override fun run() {
                onKey(r, c, Engine.TAP)
                handler.postDelayed(this, 50)
            }
        }
        repeating = run
        handler.postDelayed(run, 400)
    }

    private fun stopRepeat() {
        repeating?.let { handler.removeCallbacks(it) }
        repeating = null
    }

    private fun candidateRect(): RectF? {
        for ((r, row) in engine.rows.withIndex()) for ((c, k) in row.withIndex()) if (k.kind == "cand") return rects.getOrNull(r)?.getOrNull(c)
        return null
    }

    private fun candidateAt(x: Float): Int {
        val bar = candidateRect() ?: return -1
        val n = engine.candidates.size
        if (n == 0) return -1
        return ((x - bar.left) / (bar.width() / SLOTS)).toInt().coerceIn(0, n - 1)
    }

    // ---- drawing

    override fun onDraw(canvas: Canvas) {
        canvas.drawColor(plateColor)
        val pressed = touches.values.associateBy { it.row to it.col }
        for ((r, row) in engine.rows.withIndex()) for ((c, k) in row.withIndex()) {
            val rect = rects.getOrNull(r)?.getOrNull(c) ?: continue
            drawKey(canvas, k, rect, pressed[r to c])
        }
    }

    private fun drawKey(canvas: Canvas, k: Engine.Key, r: RectF, touch: Touch?) {
        when (k.kind) {
            "cand" -> return drawCandidates(canvas, r, touch)
            "rsv" -> {
                fill.color = (edgeColor and 0x00FFFFFF) or 0x55000000
                canvas.drawRoundRect(r, radius, radius, fill)
                return
            }
        }
        val active = touch != null || k.has(Engine.ON)
        val mod = k.kind == "f" || k.kind == "m"
        fill.color = edgeColor
        canvas.drawRoundRect(RectF(r.left, r.top + 2 * density, r.right, r.bottom), radius, radius, fill)
        fill.color = if (active) onColor else if (mod) modColor else capColor
        canvas.drawRoundRect(RectF(r.left, r.top, r.right, r.bottom - 2 * density), radius, radius, fill)
        if (k.has(Engine.LOCK)) {
            fill.color = textColor
            canvas.drawRect(r.centerX() - 6 * density, r.bottom - 7 * density, r.centerX() + 6 * density, r.bottom - 5 * density, fill)
        }

        // While flicked, the key shows what it will type.
        val flicked = touch?.dir?.takeIf { it != Engine.TAP }?.let { k.legends[it - 1] }?.takeIf { it.isNotEmpty() }
        val main = flicked ?: when {
            k.has(Engine.ICON_COPY) -> "copy"
            k.has(Engine.ICON_PASTE) -> "paste"
            k.has(Engine.ICON_BS) -> "⌫"
            k.has(Engine.ICON_ENTER) -> "↵"
            k.has(Engine.ICON_CAPS) || k.has(Engine.ICON_SHIFT) -> "⇧"
            else -> k.label
        }
        val small = k.flicks && flicked == null
        val maxSize = when {
            k.has(Engine.ICON_COPY) || k.has(Engine.ICON_PASTE) -> 12f
            mod -> 16f
            main.length > 3 -> 13f
            main.length > 1 && small -> 13f
            small -> 18f
            else -> 21f
        }
        // Fit the label in the key: no wider than its share of the key.
        val room = if (small) r.width() * 0.5f else r.width() * 0.85f
        text.typeface = if (mod) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
        text.textSize = maxSize * density
        val wText = text.measureText(main)
        if (wText > room && wText > 0) text.textSize *= room / wText
        text.color = textColor
        canvas.drawText(main, r.centerX(), r.centerY() - (text.descent() + text.ascent()) / 2 - density, text)

        if (k.has(Engine.OTHER)) {
            fill.color = accentColor
            val y = r.centerY() + text.textSize * 0.62f
            for (i in -1..1) canvas.drawCircle(r.centerX() + i * 3.5f * density, y, 1.2f * density, fill)
        }
        if (small) drawLegends(canvas, k, r)
        if (k.corner.isNotEmpty()) {
            text.color = mutedColor
            text.typeface = Typeface.DEFAULT
            text.textSize = 10 * density
            canvas.drawText(k.corner, r.right - 8 * density, r.top + 12 * density, text)
        }
        if (k.has(Engine.SMALL)) {
            text.color = flickColor
            text.typeface = Typeface.DEFAULT
            text.textSize = 9 * density
            canvas.drawText("小", r.left + 7 * density, r.top + 11 * density, text)
        }
    }

    private fun drawLegends(canvas: Canvas, k: Engine.Key, r: RectF) {
        text.color = flickColor
        text.typeface = Typeface.DEFAULT
        text.textSize = (10.5f * density).coerceAtMost(r.height() * 0.22f)
        val insetX = (r.width() * 0.13f).coerceAtMost(11 * density)
        val mid = r.centerY() - (text.descent() + text.ascent()) / 2 - density
        val (n, e, s, w) = k.legends
        canvas.drawText(w, r.left + insetX, mid, text)
        canvas.drawText(e, r.right - insetX, mid, text)
        canvas.drawText(n, r.centerX(), r.top - text.ascent() + 1 * density, text)
        canvas.drawText(s, r.centerX(), r.bottom - text.descent() - 3 * density, text)
    }

    private fun drawCandidates(canvas: Canvas, bar: RectF, touch: Touch?) {
        val cands = engine.candidates
        if (cands.isEmpty()) return
        val slot = bar.width() / SLOTS
        for ((i, c) in cands.withIndex().take(SLOTS)) {
            val r = RectF(bar.left + i * slot, bar.top, bar.left + (i + 1) * slot, bar.bottom)
            if (touch?.candidate == i) {
                fill.color = onColor
                canvas.drawRoundRect(RectF(r.left, r.top + 4 * density, r.right, r.bottom - 4 * density), radius, radius, fill)
            }
            candText.color = if (i == 0) accentColor else textColor
            candText.typeface = if (i == 0) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
            candText.textSize = 16 * density
            val shown = TextUtils.ellipsize(c, candText, slot - 8 * density, TextUtils.TruncateAt.END).toString()
            canvas.drawText(shown, r.centerX(), r.centerY() - (candText.descent() + candText.ascent()) / 2, candText)
            if (i > 0) {
                fill.color = edgeColor
                canvas.drawRect(r.left - 0.5f * density, r.top + 10 * density, r.left + 0.5f * density, r.bottom - 10 * density, fill)
            }
        }
    }

    companion object {
        /** A flick: the touch moves at least this share of the key's width (SPEC §4.2). */
        const val FLICK_THRESHOLD = 0.4f
        /** Towards a screen edge, the threshold is at most this share of the room left... */
        const val EDGE_ROOM_SHARE = 0.6f
        /** ...but never below this, so a jitter at the very edge isn't a flick. */
        const val EDGE_MIN_THRESHOLD_DP = 4f
        /** How close to the screen edge a finger's reported centre can get (measured on a Fold7). */
        const val EDGE_DEAD_DP = 20f
        const val SLOTS = 4
    }
}
