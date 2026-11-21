package com.fold.app

import android.os.Bundle
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import android.app.Activity

class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val bar = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(8, 8, 8, 8)
        }
        val view = FoldView(this)
        val status = TextView(this).apply { text = "Готово" }

        fun btn(name: String, action: () -> Unit) {
            bar.addView(Button(this).apply { text = name; setOnClickListener { action() } })
        }
        btn("Назад") { view.prevProfile() }
        btn("Далее") { view.nextProfile() }
        btn("Стереть") { view.clearTable() }
        btn("Удалить все") { view.clearAll() }
        btn("Построить") { view.rebuild() }

        view.onStatus = { s -> status.text = s; Toast.makeText(this, s, Toast.LENGTH_SHORT).show() }

        root.addView(bar)
        root.addView(view, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f))
        root.addView(status)
        setContentView(root)
    }
}
