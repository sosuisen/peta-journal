package net.sosuisen.petajournal.sync

/**
 * One sync with the cloud (ADR 031): the download, then the work of the core, then the
 * upload. The download comes first, so the core has taken in what the cloud changed before
 * it writes its pages.
 *
 * @param takeIn lets the core take in the files of the Markdown folder and write the pages
 *   that changed in the app. It returns when the core is done.
 * @param tries how often the round runs when the cloud refuses an upload.
 */
class SyncRound(
    private val downloader: Downloader,
    private val takeIn: () -> Unit,
    private val uploader: Uploader,
    private val tries: Int = 3,
) {
    /**
     * Runs once. When the cloud has a version of a file that the app has not seen, the round
     * starts again: the download brings the version, and the core puts the two together.
     * [ChangedInCloud] comes out when the cloud refused in every try.
     */
    fun run(onDownload: (done: Int, total: Int) -> Unit, onUpload: (done: Int, total: Int) -> Unit) {
        repeat(tries - 1) {
            try {
                return once(onDownload, onUpload)
            } catch (again: ChangedInCloud) {
                // The next try downloads what changed
            }
        }
        once(onDownload, onUpload)
    }

    private fun once(onDownload: (Int, Int) -> Unit, onUpload: (Int, Int) -> Unit) {
        downloader.run(onDownload)
        takeIn()
        uploader.run(onUpload)
    }
}
