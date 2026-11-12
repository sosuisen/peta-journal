package net.sosuisen.petajournal.sync

import java.io.File
import java.util.concurrent.Callable
import java.util.concurrent.ExecutionException
import java.util.concurrent.Executors

/**
 * Brings the Markdown folder of the app up to date with the folder in the cloud. It only
 * reads the cloud (ADR 030): what the cloud has is right, and what it lost is deleted here.
 *
 * @param cursorFile keeps where the last run ended, so that the next one asks only for what
 *   changed since.
 * @param threads how many files are downloaded at the same time. One after the other, a file
 *   takes about a second.
 */
class Downloader(
    private val cloud: Cloud,
    private val folder: MarkdownFolder,
    private val cursorFile: File,
    private val threads: Int = 8,
) {
    /**
     * Runs once. [onProgress] gets how many files are downloaded, of how many: first with 0,
     * then after each file, in order but from other threads.
     *
     * When the cloud or a download fails, the exception comes out and the cursor stays. The
     * next run asks for the same changes again, and downloads only what is still missing.
     */
    fun run(onProgress: (done: Int, total: Int) -> Unit) {
        val changes = cloud.changes(cursorFile.takeIf { it.exists() }?.readText())
        // By the path in lower case, as the cloud tells no case apart
        val files = LinkedHashMap<String, Change.File>()
        for (change in changes.entries) {
            val key = change.path.lowercase()
            when (change) {
                is Change.Gone -> {
                    files.keys.removeAll { it == key || it.startsWith("$key/") }
                    folder.remove(change.path)
                }
                is Change.File -> if (MarkdownFolder.keeps(key)) files[key] = change
            }
        }
        if (changes.whole) folder.keepOnly(files.keys)
        download(files.values.filter { folder.contentHash(it.path) != it.contentHash }, onProgress)
        cursorFile.writeText(changes.cursor)
    }

    private fun download(files: List<Change.File>, onProgress: (Int, Int) -> Unit) {
        onProgress(0, files.size)
        val pool = Executors.newFixedThreadPool(threads)
        try {
            var done = 0
            val told = Any()
            val downloads = files.map { file ->
                Callable {
                    folder.put(file.path) { cloud.download(file, it) }
                    synchronized(told) { onProgress(++done, files.size) }
                }
            }
            for (download in pool.invokeAll(downloads)) {
                try {
                    download.get()
                } catch (failed: ExecutionException) {
                    throw failed.cause ?: failed
                }
            }
        } finally {
            pool.shutdown()
        }
    }
}
