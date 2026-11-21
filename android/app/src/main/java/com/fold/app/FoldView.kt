package com.fold.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View
import kotlin.math.abs

data class FPick(val trace: Double, val time: Double)
data class FProfile(
    val x0: Double, val y0: Double, val x1: Double, val y1: Double,
    val picks: MutableList<FPick> = mutableListOf()
)

class FoldView @JvmOverloads constructor(
    context: Context, attrs: AttributeSet? = null
) : View(context, attrs) {

    var velocity = 1200f
    var quality = 95f
    var onStatus: ((String) -> Unit)? = null
    var profiles = mutableListOf(
        FProfile(100.0, 100.0, 3100.0, 100.0, mutableListOf(
            FPick(1.0, 3.465), FPick(3.0, 3.474), FPick(5.0, 3.290),
            FPick(7.0, 3.524), FPick(10.0, 3.520)
        ))
    )
    var selected = 0

    private var result: FloatArray? = null
    private var mapW = 0
    private var mapH = 0
    private var dragStart: Pair<Float, Float>? = null
    private var dragCur: Pair<Float, Float>? = null

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { isFilterBitmap = true }
    private val line = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(30, 90, 210); strokeWidth = 4f
    }
    private val selectedLine = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(230, 50, 40); strokeWidth = 6f
    }
    private val grid = Paint().apply { color = Color.rgb(150, 160, 170); strokeWidth = 2f }
    private val label = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(40, 45, 50); textSize = 28f
    }

    private val mapTop get() = height * 0.12f
    private val mapBottom get() = height * 0.62f
    private val mapLeft get() = width * 0.22f
    private val mapRight get() = width * 0.96f
    private val secTop get() = height * 0.68f
    private val secBottom get() = height * 0.93f

    fun rebuild() {
        val data = mutableListOf<Float>()
        data.add(profiles.size.toFloat())
        for (p in profiles) {
            data.add(p.x0.toFloat()); data.add(p.y0.toFloat())
            data.add(p.x1.toFloat()); data.add(p.y1.toFloat())
            data.add(p.picks.size.toFloat())
            for (pk in p.picks) {
                data.add(pk.trace.toFloat()); data.add(pk.time.toFloat())
            }
        }
        result = FoldNative.buildMap(data.toFloatArray(), velocity, quality)
        invalidate()
    }

    fun clearTable() {
        if (profiles.isNotEmpty()) {
            profiles[selected].picks.clear()
            rebuild()
            onStatus?.invoke("Таблица корреляции очищена")
        }
    }

    fun clearAll() {
        profiles.clear(); selected = 0; rebuild()
        onStatus?.invoke("Все профили удалены")
    }

    fun nextProfile() {
        if (selected + 1 < profiles.size) { selected++; invalidate() }
    }

    fun prevProfile() {
        if (selected > 0) { selected--; invalidate() }
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        rebuild()
    }

    override fun onDraw(c: Canvas) {
        super.onDraw(c)
        c.drawColor(Color.rgb(245, 245, 245))

        label.textSize = 30f
        c.drawText("Построение структурной карты отражающего горизонта", width * 0.12f, height * 0.055f, label)
        label.textSize = 24f
        c.drawText("Параметры структуры", 16f, height * 0.10f, label)
        c.drawText("V ср  ${velocity.toInt()}", 16f, height * 0.14f, label)
        c.drawText("Координаты профиля", 16f, height * 0.19f, label)
        c.drawText("Профиль ${selected + 1}", 24f, height * 0.23f, label)
        c.drawText("Таблица корреляции", 16f, height * 0.36f, label)
        c.drawText("Качество изображения ${quality.toInt()}", 16f, height * 0.95f, label)

        val r = result
        if (r != null && r.size >= 2) {
            mapW = r[0].toInt(); mapH = r[1].toInt()
            val depths = r.copyOfRange(2, r.size)
            var min = Float.POSITIVE_INFINITY
            var max = Float.NEGATIVE_INFINITY
            for (v in depths) if (v > -9000f) { min = min.coerceAtMost(v); max = max.coerceAtLeast(v) }
            if (!min.isFinite()) { min = 0f; max = 1f }
            if (max - min < 0.001f) max = min + 1f
            val bmp = Bitmap.createBitmap(mapW, mapH, Bitmap.Config.ARGB_8888)
            val px = IntArray(mapW * mapH)
            for (i in depths.indices) {
                val d = depths[i]
                px[i] = if (d > -9000f) palette((d - min) / (max - min)) else Color.rgb(220, 228, 232)
            }
            bmp.setPixels(px, 0, mapW, 0, 0, mapW, mapH)
            c.drawBitmap(bmp, null, android.graphics.RectF(mapLeft, mapTop, mapRight, mapBottom), paint)
            bmp.recycle()
        } else {
            c.drawRect(mapLeft, mapTop, mapRight, mapBottom, Paint().apply { color = Color.rgb(230, 238, 242) })
        }

        // сетка карты
        for (i in 0..8) {
            val x = mapLeft + (mapRight - mapLeft) * i / 8f
            c.drawLine(x, mapTop, x, mapBottom, grid)
        }
        for (i in 0..6) {
            val y = mapTop + (mapBottom - mapTop) * i / 6f
            c.drawLine(mapLeft, y, mapRight, y, grid)
        }
        c.drawText("X, м", mapRight - 120f, mapTop - 12f, label)

        for ((i, p) in profiles.withIndex()) {
            val a = mapPoint(p.x0, p.y0)
            val b = mapPoint(p.x1, p.y1)
            val pn = if (i == selected) selectedLine else line
            c.drawLine(a.first, a.second, b.first, b.second, pn)
            c.drawCircle(a.first, a.second, 8f, pn)
            c.drawCircle(b.first, b.second, 8f, pn)
        }
        val ds = dragStart
        val dc = dragCur
        if (ds != null && dc != null) {
            c.drawLine(ds.first, ds.second, dc.first, dc.second, selectedLine)
        }

        // временной разрез
        label.textSize = 26f
        c.drawText("Временной разрез", mapLeft, secTop - 12f, label)
        c.drawText("трассы", mapRight - 90f, secTop - 12f, label)
        c.drawRect(mapLeft, secTop, mapRight, secBottom, Paint().apply { color = Color.WHITE; style = Paint.Style.STROKE; strokeWidth = 2f })
        for (i in 1..10) {
            val x = mapLeft + (mapRight - mapLeft) * i / 11f
            c.drawLine(x, secTop, x, secBottom, grid)
            c.drawText(i.toString(), x - 8f, secBottom + 34f, label)
        }
        val picks = profiles.getOrNull(selected)?.picks.orEmpty()
        val pts = picks.map { pk ->
            val x = mapLeft + (mapRight - mapLeft) * (pk.trace / 11.0).toFloat()
            val y = secBottom - ((pk.time - 3.2) / 0.4).coerceIn(0.0, 1.0).toFloat() * (secBottom - secTop)
            x to y
        }
        for (i in 1 until pts.size) {
            c.drawLine(pts[i - 1].first, pts[i - 1].second, pts[i].first, pts[i].second, selectedLine)
        }
        for (p in pts) c.drawCircle(p.first, p.second, 8f, selectedLine)

        label.textSize = 20f
        c.drawText("Viktor Romanov, 2007", mapRight - 260f, height - 14f, label)
    }

    private fun mapPoint(x: Double, y: Double): Pair<Float, Float> =
        mapLeft + (x / 8000.0).toFloat() * (mapRight - mapLeft) to
                mapTop + (y / 4000.0).toFloat() * (mapBottom - mapTop)

    override fun onTouchEvent(e: MotionEvent): Boolean {
        val x = e.x; val y = e.y
        when (e.action) {
            MotionEvent.ACTION_DOWN -> {
                if (x in mapLeft..mapRight && y in mapTop..mapBottom) {
                    dragStart = x to y; dragCur = x to y
                } else if (x in mapLeft..mapRight && y in secTop..secBottom) {
                    val trace = ((x - mapLeft) / (mapRight - mapLeft) * 11.0)
                    val time = 3.2 + (1.0 - (y - secTop) / (secBottom - secTop)) * 0.4
                    if (profiles.isEmpty()) profiles.add(FProfile(0.0, 2000.0, 8000.0, 2000.0))
                    profiles[selected].picks.add(FPick(trace, time))
                    profiles[selected].picks.sortBy { it.trace }
                    rebuild()
                    onStatus?.invoke("Пик: трасса %.1f, t %.3f с".format(trace, time))
                }
                invalidate()
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (dragStart != null) { dragCur = x to y; invalidate() }
                return true
            }
            MotionEvent.ACTION_UP -> {
                val a = dragStart; val b = dragCur
                if (a != null && b != null && (abs(a.first - b.first) + abs(a.second - b.second) > 20f)) {
                    val x0 = ((a.first - mapLeft) / (mapRight - mapLeft) * 8000.0)
                    val y0 = ((a.second - mapTop) / (mapBottom - mapTop) * 4000.0)
                    val x1 = ((b.first - mapLeft) / (mapRight - mapLeft) * 8000.0)
                    val y1 = ((b.second - mapTop) / (mapBottom - mapTop) * 4000.0)
                    selected = profiles.size
                    profiles.add(FProfile(x0, y0, x1, y1, mutableListOf(
                        FPick(1.0, 3.465), FPick(5.0, 3.520), FPick(10.0, 3.500)
                    )))
                    rebuild()
                    onStatus?.invoke("Профиль ${selected + 1} добавлен")
                }
                dragStart = null; dragCur = null; invalidate()
                return true
            }
        }
        return super.onTouchEvent(e)
    }

    private fun palette(u: Float): Int {
        val t = u.coerceIn(0f, 1f)
        val stops = arrayOf(
            0f to intArrayOf(82, 203, 183),
            0.25f to intArrayOf(80, 203, 102),
            0.5f to intArrayOf(134, 203, 62),
            0.75f to intArrayOf(185, 203, 62),
            1f to intArrayOf(202, 171, 62)
        )
        for (i in 0 until stops.size - 1) {
            if (t <= stops[i + 1].first) {
                val k = (t - stops[i].first) / (stops[i + 1].first - stops[i].first)
                val a = stops[i].second; val b = stops[i + 1].second
                return Color.rgb(
                    (a[0] + (b[0] - a[0]) * k).toInt(),
                    (a[1] + (b[1] - a[1]) * k).toInt(),
                    (a[2] + (b[2] - a[2]) * k).toInt()
                )
            }
        }
        return Color.rgb(202, 171, 62)
    }
}
