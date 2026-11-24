package com.fold.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View
import kotlin.math.abs

data class FPick(val trace: Double, val time: Double)
data class FProfile(
    val x0: Double, val y0: Double, val x1: Double, val y1: Double,
    val picks: MutableList<FPick> = mutableListOf()
)

private const val MAP_X_M = 8000.0
private const val MAP_Y_M = 4000.0
private const val SECTION_W = 800
private const val SECTION_H = 201
// 0x468780 / desktop GUI: пики и клик по Image2 используют 11 условных трасс,
// независимо от числа отсчётов, нарисованных в Image2.
private const val SECTION_PICK_TRACES = 11.0

class FoldView @JvmOverloads constructor(
    context: Context, attrs: AttributeSet? = null
) : View(context, attrs) {

    var velocity = 1500f
    var quality = 3f
    var angle = 0f
    var onStatus: ((String) -> Unit)? = null
    var onProfileChanged: ((FProfile?) -> Unit)? = null

    var profiles = mutableListOf(
        FProfile(100.0, 100.0, 3100.0, 100.0, mutableListOf(
            FPick(1.0, 3.465), FPick(3.0, 3.474), FPick(5.0, 3.290),
            FPick(7.0, 3.524), FPick(10.0, 3.520)
        ))
    )
    var selected = 0

    private var mapBitmap: Bitmap? = null
    private var sectionBitmap: Bitmap? = null
    private var sectionTMin = 0f
    private var sectionTMax = 1f
    private var sectionTraces = 0
    private var dragStart: Pair<Float, Float>? = null
    private var dragCur: Pair<Float, Float>? = null

    private val bitmapPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { isFilterBitmap = false }
    private val line = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(30, 90, 210); strokeWidth = 4f
    }
    private val selectedLine = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(230, 50, 40); strokeWidth = 6f
    }
    private val grid = Paint().apply { color = Color.rgb(200, 200, 200); strokeWidth = 2f }
    private val border = Paint().apply {
        color = Color.rgb(140, 140, 140); style = Paint.Style.STROKE; strokeWidth = 2f
    }
    private val label = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(40, 45, 50); textSize = 26f
    }
    private val small = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(70, 75, 80); textSize = 20f
    }
    private val pickPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.RED; style = Paint.Style.FILL
    }

    fun setVelocity(value: Float) {
        velocity = value.coerceIn(1000f, 7000f)
        rebuild()
    }

    fun setQuality(value: Float) {
        quality = value.coerceIn(0.1f, 9.99f)
        rebuild()
    }

    fun setAngle(value: Float) {
        angle = value.coerceIn(0f, 360f)
        rebuild()
    }

    fun setCoordinate(index: Int, value: Double) {
        val p = profiles.getOrNull(selected) ?: return
        val v = when (index) {
            0, 2 -> value.coerceIn(0.0, MAP_X_M)
            else -> value.coerceIn(0.0, MAP_Y_M)
        }
        profiles[selected] = when (index) {
            0 -> p.copy(x0 = v)
            1 -> p.copy(y0 = v)
            2 -> p.copy(x1 = v)
            else -> p.copy(y1 = v)
        }
        rebuild()
        onProfileChanged?.invoke(profiles[selected])
    }

    fun rebuild() {
        val map = FoldNative.buildMapPixels(velocity, quality, angle)
        if (map == null || map.size < 2) {
            onStatus?.invoke("Не удалось построить карту")
            return
        }
        val mw = map[0]
        val mh = map[1]
        val mpx = map.copyOfRange(2, map.size)
        if (mw <= 0 || mh <= 0 || mpx.size < mw * mh) {
            onStatus?.invoke("Некорректный буфер карты")
            return
        }
        mapBitmap = updateBitmap(mapBitmap, mw, mh, mpx)

        val p = profiles.getOrNull(selected)
        if (p == null) {
            recycleBitmap(sectionBitmap)
            sectionBitmap = null
            sectionTraces = 0
        } else {
            val sec = FoldNative.buildSectionPixels(
                p.x0.toFloat(), p.y0.toFloat(), p.x1.toFloat(), p.y1.toFloat(),
                velocity, quality, angle
            )
            if (sec == null || sec.size < 5) {
                recycleBitmap(sectionBitmap)
                sectionBitmap = null
                sectionTraces = 0
                onStatus?.invoke("Не удалось построить разрез")
            } else {
                val w = sec[0]
                val h = sec[1]
                val px = sec.copyOfRange(5, sec.size)
                if (w <= 0 || h <= 0 || px.size < w * h) {
                    recycleBitmap(sectionBitmap)
                    sectionBitmap = null
                    sectionTraces = 0
                    onStatus?.invoke("Некорректный буфер разреза")
                } else {
                    sectionTMin = Float.fromBits(sec[2])
                    sectionTMax = Float.fromBits(sec[3])
                    sectionTraces = sec[4]
                    sectionBitmap = updateBitmap(sectionBitmap, w, h, px)
                }
            }
        }
        invalidate()
    }

    private fun recycleBitmap(b: Bitmap?) {
        if (b != null && !b.isRecycled) {
            b.recycle()
        }
    }

    private fun updateBitmap(old: Bitmap?, w: Int, h: Int, px: IntArray): Bitmap {
        val bmp = if (old != null && old.width == w && old.height == h) {
            old
        } else {
            // Сначала выделяем новый буфер, чтобы не терять старый при OOM.
            val fresh = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
            recycleBitmap(old)
            fresh
        }
        bmp.setPixels(px, 0, w, 0, 0, w, h)
        return bmp
    }

    fun clearTable() {
        profiles.getOrNull(selected)?.picks?.clear()
        rebuild()
        onStatus?.invoke("Таблица корреляции очищена")
    }

    fun clearAll() {
        profiles.clear()
        selected = 0
        recycleBitmap(sectionBitmap)
        sectionBitmap = null
        rebuild()
        onProfileChanged?.invoke(null)
        onStatus?.invoke("Все профили удалены")
    }

    fun nextProfile() {
        if (selected + 1 < profiles.size) {
            selected++
            rebuild()
            onProfileChanged?.invoke(profiles[selected])
        }
    }

    fun prevProfile() {
        if (selected > 0) {
            selected--
            rebuild()
            onProfileChanged?.invoke(profiles[selected])
        }
    }

    private fun fitRect(l: Float, t: Float, r: Float, b: Float, aspect: Float): RectF {
        val aw = (r - l).coerceAtLeast(1f)
        val ah = (b - t).coerceAtLeast(1f)
        var w = aw
        var h = w / aspect
        if (h > ah) {
            h = ah
            w = h * aspect
        }
        return RectF(l + (aw - w) / 2f, t + (ah - h) / 2f,
            l + (aw + w) / 2f, t + (ah + h) / 2f)
    }

    private fun mapRect() = fitRect(width * 0.03f, height * 0.17f,
        width * 0.97f, height * 0.63f, 2f)

    private fun sectionRect() = fitRect(width * 0.03f, height * 0.70f,
        width * 0.97f, height * 0.94f, SECTION_W.toFloat() / SECTION_H)

    private fun mapPoint(x: Double, y: Double): Pair<Float, Float> {
        val r = mapRect()
        return (r.left + (x / MAP_X_M).toFloat() * r.width()) to
                (r.bottom - (y / MAP_Y_M).toFloat() * r.height())
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        rebuild()
    }

    override fun onDraw(c: Canvas) {
        super.onDraw(c)
        c.drawColor(Color.rgb(250, 250, 250))

        label.textSize = 28f
        c.drawText("Построение структурной карты отражающего горизонта", 12f, 28f, label)
        small.textSize = 21f
        c.drawText("V ср ${velocity.toInt()}   Качество ${quality.toInt()}   Угол ${angle.toInt()}°",
            12f, 56f, small)
        c.drawText("Координаты профиля / Профиль ${selected + 1}", 12f, 82f, small)

        val mr = mapRect()
        val bmp = mapBitmap
        if (bmp != null) {
            c.drawBitmap(bmp, null, mr, bitmapPaint)
        } else {
            c.drawRect(mr, Paint().apply { color = Color.rgb(230, 238, 242) })
        }
        for (i in 0..8) {
            val x = mr.left + mr.width() * i / 8f
            c.drawLine(x, mr.top, x, mr.bottom, grid)
        }
        for (i in 0..10) {
            val y = mr.top + mr.height() * i / 10f
            c.drawLine(mr.left, y, mr.right, y, grid)
        }
        c.drawText("X, м", mr.right - 70f, mr.top - 8f, small)

        for ((i, p) in profiles.withIndex()) {
            val a = mapPoint(p.x0, p.y0)
            val b = mapPoint(p.x1, p.y1)
            val pn = if (i == selected) selectedLine else line
            c.drawLine(a.first, a.second, b.first, b.second, pn)
            c.drawCircle(a.first, a.second, 7f, pn)
            c.drawCircle(b.first, b.second, 7f, pn)
        }
        val ds = dragStart
        val dc = dragCur
        if (ds != null && dc != null) {
            c.drawLine(ds.first, ds.second, dc.first, dc.second, selectedLine)
        }

        val sr = sectionRect()
        val sb = sectionBitmap
        if (sb != null) {
            c.drawBitmap(sb, null, sr, bitmapPaint)
        } else {
            c.drawRect(sr, Paint().apply { color = Color.WHITE })
        }
        c.drawRect(sr, border)
        small.textSize = 20f
        c.drawText("Временной разрез", sr.left, sr.top - 10f, small)
        c.drawText("трассы · $sectionTraces", sr.right - 170f, sr.top - 10f, small)

        // 0x468780: HACHA — горизонтальная красная черточка 12×4 в pix Image2.
        val span = (sectionTMax - sectionTMin).coerceAtLeast(1e-6f)
        for (pk in profiles.getOrNull(selected)?.picks.orEmpty()) {
            val px = (pk.trace / SECTION_PICK_TRACES * SECTION_W).toFloat()
            val py = (((pk.time - sectionTMin) / span) * SECTION_H)
                .coerceIn(0.0, SECTION_H.toDouble()).toFloat()
            if (px < -6f || px > SECTION_W + 6f) continue
            c.drawRect(RectF(
                sr.left + (px - 6f) / SECTION_W * sr.width(),
                sr.top + (py - 2f) / SECTION_H * sr.height(),
                sr.left + (px + 6f) / SECTION_W * sr.width(),
                sr.top + (py + 2f) / SECTION_H * sr.height()
            ), pickPaint)
        }

        small.textSize = 18f
        c.drawText("Rust version Fold by Lion 2026", 12f, height - 8f, small)
    }

    override fun onTouchEvent(e: MotionEvent): Boolean {
        val x = e.x
        val y = e.y
        val mr = mapRect()
        val sr = sectionRect()
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                if (mr.contains(x, y)) {
                    dragStart = x to y
                    dragCur = x to y
                    // Родительский ScrollView не должен перехватывать рисование профиля.
                    parent.requestDisallowInterceptTouchEvent(true)
                } else if (sr.contains(x, y)) {
                    val trace = ((x - sr.left) / sr.width() * SECTION_PICK_TRACES)
                        .coerceIn(0.0, SECTION_PICK_TRACES - 1.0)
                    val py = ((y - sr.top) / sr.height() * SECTION_H)
                        .coerceIn(0f, SECTION_H.toFloat())
                    val time = sectionTMin + (py / SECTION_H) * (sectionTMax - sectionTMin)
                    if (profiles.isEmpty()) {
                        profiles.add(FProfile(100.0, 2000.0, 3100.0, 2000.0))
                        onProfileChanged?.invoke(profiles[selected])
                    }
                    profiles[selected].picks.add(FPick(trace, time.toDouble()))
                    profiles[selected].picks.sortBy { it.trace }
                    rebuild()
                    onStatus?.invoke("Пик: трасса %.1f, t %.3f с".format(trace, time))
                }
                invalidate()
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (dragStart != null) {
                    dragCur = x to y
                    invalidate()
                }
                return true
            }
            MotionEvent.ACTION_UP -> {
                val a = dragStart
                val b = dragCur
                if (a != null && b != null &&
                    abs(a.first - b.first) + abs(a.second - b.second) > 20f
                ) {
                    val x0 = ((a.first - mr.left) / mr.width() * MAP_X_M)
                        .coerceIn(0.0, MAP_X_M)
                    val y0 = (MAP_Y_M - (a.second - mr.top) / mr.height() * MAP_Y_M)
                        .coerceIn(0.0, MAP_Y_M)
                    val x1 = ((b.first - mr.left) / mr.width() * MAP_X_M)
                        .coerceIn(0.0, MAP_X_M)
                    val y1 = (MAP_Y_M - (b.second - mr.top) / mr.height() * MAP_Y_M)
                        .coerceIn(0.0, MAP_Y_M)
                    selected = profiles.size
                    profiles.add(FProfile(x0, y0, x1, y1))
                    rebuild()
                    onProfileChanged?.invoke(profiles[selected])
                    onStatus?.invoke("Профиль ${selected + 1} добавлен")
                }
                dragStart = null
                dragCur = null
                parent.requestDisallowInterceptTouchEvent(false)
                invalidate()
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                dragStart = null
                dragCur = null
                parent.requestDisallowInterceptTouchEvent(false)
                invalidate()
                return true
            }
        }
        return super.onTouchEvent(e)
    }
}
