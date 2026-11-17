package com.sosuisha.petajournal.shell

import java.io.File

/**
 * The file or folder at the path under [root], however its name is spelled there. The path
 * that a page writes for its image may differ from it in case. Empty parts of the path are
 * skipped, as the sync of the core and the iOS shell do (ADR 054).
 */
internal fun findWithoutCase(root: File, path: String): File? =
    path.split('/').filter { it.isNotEmpty() }.fold<String, File?>(root) { dir, name -> dir?.let { child(it, name) } }

private fun child(dir: File, name: String): File? =
    dir.listFiles()?.firstOrNull { it.name.equals(name, ignoreCase = true) }
