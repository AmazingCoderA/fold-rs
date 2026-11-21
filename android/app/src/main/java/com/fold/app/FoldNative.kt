package com.fold.app

object FoldNative {
    init {
        System.loadLibrary("fold_android")
    }

    /**
     * data: [n, x0,y0,x1,y1,npicks, (trace,time)*npicks, ...]
     * return: [width, height, depth...] (NODATA = -9999)
     */
    external fun buildMap(data: FloatArray, velocity: Float, quality: Float): FloatArray
}
