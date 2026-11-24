package com.fold.app

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.view.ViewGroup
import android.view.inputmethod.EditorInfo
import android.widget.Button
import android.widget.EditText
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast

class MainActivity : Activity() {
    private lateinit var coordFields: Array<EditText>

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()

        val scroll = ScrollView(this).apply {
            isFillViewport = true
            setBackgroundColor(Color.WHITE)
        }
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(8), dp(12), dp(8))
        }

        val title = TextView(this).apply {
            text = "Fold — структурная карта"
            textSize = 22f
            setTextColor(Color.rgb(0, 0, 128))
            setPadding(0, 0, 0, dp(2))
        }
        val version = TextView(this).apply {
            text = "Rust version ${appVersion()}"
            textSize = 15f
            setTextColor(Color.rgb(90, 90, 90))
            setPadding(0, 0, 0, dp(6))
        }
        root.addView(title)
        root.addView(version)

        val buttonsScroll = HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = true }
        val bar = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        buttonsScroll.addView(bar)

        val view = FoldView(this)
        fun button(name: String, action: () -> Unit) {
            bar.addView(Button(this).apply {
                text = name
                isAllCaps = false
                setPadding(dp(10), dp(6), dp(10), dp(6))
                setOnClickListener { action() }
            })
        }
        button("Назад") { view.prevProfile() }
        button("Далее") { view.nextProfile() }
        button("Построить") { view.rebuild() }
        button("Стереть") { view.clearTable() }
        button("Удалить все") { view.clearAll() }
        button("Обновить") { openLatestRelease() }
        root.addView(buttonsScroll)

        val params = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, dp(6), 0, dp(2))
        }
        val vField = numberField("1500") { view.setVelocity(it.toFloatOrNull() ?: 1500f) }
        val qField = numberField("3") { view.setQuality(it.toFloatOrNull() ?: 3f) }
        val aField = numberField("0") { view.setAngle(it.toFloatOrNull() ?: 0f) }
        params.addView(labeled("V", vField))
        params.addView(labeled("Качество", qField))
        params.addView(labeled("Угол", aField))
        root.addView(params)

        val coordScroll = HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = true }
        val coordBar = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        coordFields = arrayOf(
            numberField("100") { view.setCoordinate(0, it.toDoubleOrNull() ?: 100.0) },
            numberField("100") { view.setCoordinate(1, it.toDoubleOrNull() ?: 100.0) },
            numberField("3100") { view.setCoordinate(2, it.toDoubleOrNull() ?: 3100.0) },
            numberField("100") { view.setCoordinate(3, it.toDoubleOrNull() ?: 100.0) }
        )
        for ((i, name) in arrayOf("X1", "Y1", "X2", "Y2").withIndex()) {
            coordBar.addView(labeled(name, coordFields[i]))
        }
        coordScroll.addView(coordBar)
        root.addView(coordScroll)

        val status = TextView(this).apply {
            text = "Готово"
            textSize = 16f
            minHeight = dp(28)
            setPadding(0, dp(4), 0, dp(4))
        }
        view.onStatus = { s ->
            status.text = s
            Toast.makeText(this, s, Toast.LENGTH_SHORT).show()
        }
        view.onProfileChanged = { p ->
            val vals = p?.let { doubleArrayOf(it.x0, it.y0, it.x1, it.y1) }
                ?: doubleArrayOf(0.0, 0.0, 0.0, 0.0)
            for (i in coordFields.indices) {
                coordFields[i].setText("%.0f".format(vals[i]))
            }
        }

        root.addView(
            view,
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(500))
        )
        root.addView(status)
        root.addView(TextView(this).apply {
            text = "Rust version Fold by Lion 2026"
            textSize = 14f
            setTextColor(Color.rgb(110, 110, 110))
            gravity = Gravity.CENTER
            setPadding(0, dp(6), 0, dp(4))
        })

        scroll.addView(root)
        setContentView(scroll)
        view.rebuild()
    }

    private fun numberField(initial: String, changed: (String) -> Unit): EditText =
        EditText(this).apply {
            setText(initial)
            setSingleLine(true)
            inputType = InputType.TYPE_CLASS_NUMBER or
                    InputType.TYPE_NUMBER_FLAG_DECIMAL or
                    InputType.TYPE_NUMBER_FLAG_SIGNED
            imeOptions = EditorInfo.IME_ACTION_DONE
            layoutParams = LinearLayout.LayoutParams(190, ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                setMargins(8, 0, 8, 0)
            }
            setOnFocusChangeListener { _, hasFocus ->
                if (!hasFocus) changed(text.toString())
            }
            setOnEditorActionListener { _, actionId, _ ->
                if (actionId == EditorInfo.IME_ACTION_DONE) {
                    changed(text.toString())
                    clearFocus()
                    true
                } else {
                    false
                }
            }
        }

    private fun labeled(name: String, field: EditText): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(this@MainActivity).apply {
                text = name
                textSize = 13f
                setTextColor(Color.rgb(90, 90, 90))
            })
            addView(field)
        }

    private fun appVersion(): String = try {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val info = packageManager.getPackageInfo(
                packageName,
                android.content.pm.PackageInfo.PackageInfoFlags.of(0)
            )
            info.versionName ?: "?"
        } else {
            @Suppress("DEPRECATION")
            packageManager.getPackageInfo(packageName, 0).versionName ?: "?"
        }
    } catch (_: Exception) {
        "?"
    }

    private fun openLatestRelease() {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/AmazingCoderA/fold-rs/releases/latest"))
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            Toast.makeText(this, "Не найден браузер", Toast.LENGTH_SHORT).show()
        }
    }
}
