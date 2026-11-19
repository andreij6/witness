package app.witness.core.domain.usecase

import app.witness.core.domain.port.CaseBundle
import app.witness.core.domain.port.CaseBundleCodec
import app.witness.core.model.Submission

object CaseBundleCbor : CaseBundleCodec {
    const val VERSION: Int = 1
    const val SUBMISSION_VERSION: Int = 1
    const val MAX_ITEMS: Int = 64
    const val MAX_TITLE: Int = 512
    const val MAX_SUMMARY: Int = 16 * 1024

    private const val MAJOR_UINT = 0
    private const val MAJOR_BYTES = 2
    private const val MAJOR_TEXT = 3
    private const val MAJOR_ARRAY = 4
    private const val MAJOR_MAP = 5

    override fun encode(bundle: CaseBundle): ByteArray {
        val title = bundle.title.encodeToByteArray()
        val summary = bundle.summary?.encodeToByteArray()
        require(title.size <= MAX_TITLE) { "title too long" }
        require(summary == null || summary.size <= MAX_SUMMARY) { "summary too long" }
        require(bundle.items.size in 1..MAX_ITEMS) { "a case bundle holds 1..$MAX_ITEMS items" }
        for (item in bundle.items) {
            require(item.jpeg.size > 0) { "empty image" }
            require(item.submission == null || item.submission.recordId == item.recordId) { "submission belongs to another record" }
        }
        val w = Writer(size(title, summary, bundle))
        w.head(MAJOR_MAP, if (summary == null) 3 else 4)
        w.uint(0); w.uint(VERSION.toLong())
        w.uint(1); w.blob(MAJOR_TEXT, title)
        if (summary != null) {
            w.uint(2); w.blob(MAJOR_TEXT, summary)
        }
        w.uint(3); w.head(MAJOR_ARRAY, bundle.items.size.toLong())
        for (item in bundle.items) {
            val s = item.submission
            w.head(MAJOR_MAP, if (s == null) 2 else 3)
            w.uint(0); w.blob(MAJOR_BYTES, item.recordId.toBytes())
            w.uint(1); w.head(MAJOR_BYTES, item.jpeg.size.toLong()); w.put(item.jpeg)
            if (s != null) {
                w.uint(2); submission(w, s)
            }
        }
        check(w.full) { "size mismatch" }
        return w.out
    }

    private fun submission(w: Writer, s: Submission) {
        w.head(MAJOR_MAP, 4)
        w.uint(0); w.uint(SUBMISSION_VERSION.toLong())
        for ((key, part) in listOf(1 to s.claimCbor, 2 to s.c2paClaimCbor, 3 to s.coseSign1)) {
            w.uint(key.toLong()); w.head(MAJOR_BYTES, part.size.toLong()); w.put(part)
        }
    }

    private fun size(title: ByteArray, summary: ByteArray?, bundle: CaseBundle): Int {
        var n = headSize(3) + 2 * 1 + headSize(VERSION.toLong()) + headSize(title.size.toLong()) + title.size
        if (summary != null) n += 1 + headSize(summary.size.toLong()) + summary.size
        n += 1 + headSize(bundle.items.size.toLong())
        for (item in bundle.items) {
            n += 1 + 1 + headSize(10) + 10 + 1 + headSize(item.jpeg.size.toLong()) + item.jpeg.size
            val s = item.submission ?: continue
            n += 1 + 1 + 1 + headSize(SUBMISSION_VERSION.toLong())
            for (part in listOf(s.claimCbor, s.c2paClaimCbor, s.coseSign1)) n += 1 + headSize(part.size.toLong()) + part.size
        }
        return n
    }

    private fun headSize(arg: Long): Int = when {
        arg < 24 -> 1
        arg <= 0xff -> 2
        arg <= 0xffff -> 3
        arg <= 0xffff_ffffL -> 5
        else -> 9
    }

    private class Writer(size: Int) {
        val out = ByteArray(size)
        private var p = 0
        val full: Boolean get() = p == out.size

        fun head(major: Int, arg: Long) {
            val m = major shl 5
            when (headSize(arg)) {
                1 -> out[p++] = (m or arg.toInt()).toByte()
                2 -> { out[p++] = (m or 24).toByte(); be(arg, 1) }
                3 -> { out[p++] = (m or 25).toByte(); be(arg, 2) }
                5 -> { out[p++] = (m or 26).toByte(); be(arg, 4) }
                else -> { out[p++] = (m or 27).toByte(); be(arg, 8) }
            }
        }

        fun uint(v: Long) = head(MAJOR_UINT, v)

        fun blob(major: Int, bytes: ByteArray) {
            head(major, bytes.size.toLong())
            bytes.copyInto(out, p)
            p += bytes.size
        }

        fun put(bytes: app.witness.core.model.ByteString) {
            bytes.copyInto(out, p)
            p += bytes.size
        }

        private fun be(v: Long, n: Int) {
            for (i in n - 1 downTo 0) out[p++] = (v ushr (8 * i)).toByte()
        }
    }
}
