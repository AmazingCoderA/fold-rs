package com.fold.app

object FoldNative {
    init {
        System.loadLibrary("fold_android")
    }

    /**
     * Legacy API, оставлен для совместимости со старыми вызовами.
     * data: [n, x0,y0,x1,y1,npicks, (trace,time)*npicks, ...]
     * return: [width, height, depth...] (NODATA = -9999)
     */
    external fun buildMap(data: FloatArray, velocity: Float, quality: Float): FloatArray

    /** [800, 400, ARGB...] — карта в пикселях Image1, как в desktop GUI. */
    external fun buildMapPixels(velocity: Float, quality: Float, angle: Float): IntArray

    /**
     * [800, 201, t_min_bits, t_max_bits, n_traces, ARGB...] — разрез Image2.
     * Время в битах Float: Float.fromBits(header[2..3]).
     */
    external fun buildSectionPixels(
        x0: Float, y0: Float, x1: Float, y1: Float,
        velocity: Float, quality: Float, angle: Float
    ): IntArray
}
