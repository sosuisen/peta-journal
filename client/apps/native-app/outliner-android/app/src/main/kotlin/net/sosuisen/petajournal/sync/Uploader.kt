package net.sosuisen.petajournal.sync

/**
 * Brings the pages that changed in the app to the folder in the cloud (ADR 031). It uploads
 * the Markdown files whose content is not what the cloud had when the app last heard of it.
 * It deletes and moves nothing in the cloud.
 */
class Uploader(private val cloud: Cloud, private val folder: MarkdownFolder, private val record: CloudRecord) {
    /**
     * Runs once, after a download and after the core wrote its pages. [onProgress] gets how
     * many files are uploaded, of how many: first with 0, then after each file.
     *
     * [ChangedInCloud] comes out when the cloud has a version of a file that the app has not
     * seen: the file there is not written over, and the sync is to start again with a
     * download. What was uploaded before that, or before another failure, stays uploaded.
     */
    fun run(onProgress: (done: Int, total: Int) -> Unit) {
        val changed = folder.pages().filter { folder.contentHash(it, cloud::contentHash) != record[it]?.contentHash }
        onProgress(0, changed.size)
        try {
            changed.forEachIndexed { done, path ->
                record.put(cloud.upload(path, folder.file(path), over = record[path]?.remote))
                onProgress(done + 1, changed.size)
            }
        } finally {
            record.save()
        }
    }
}
