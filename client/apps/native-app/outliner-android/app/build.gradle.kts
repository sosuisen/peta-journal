plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.ncorti.ktfmt.gradle")
    id("pl.droidsonroids.pitest")
    jacoco
}

// The formatter of the Kotlin code: `gradlew ktfmtFormat` writes it, and `gradlew ktfmtCheck`
// fails when a file is not formatted. The Kotlin style keeps the indent of 4 spaces
ktfmt { kotlinLangStyle() }

// generate.ps1 writes the Kotlin of the border. It is not kept in the repository, and the next
// generate.ps1 would undo any formatting
tasks.withType<com.ncorti.ktfmt.gradle.tasks.KtfmtBaseTask>().configureEach {
    exclude { it.file.invariantSeparatorsPath.contains("/generated/") }
}

// The App key of the Dropbox app and the Markdown folder in Dropbox, such as `/PetaJournal`.
// They come from outside and are not kept in the repository (ADR 030):
// `gradlew -PdropboxKey=... -PdropboxFolder=... :app:assembleDebug`, or the same two names in
// the gradle.properties of the user. Without a key, the app is built without the sync
val dropboxKey = (findProperty("dropboxKey") as String?).orEmpty()
val dropboxFolder = (findProperty("dropboxFolder") as String?).orEmpty()

// The coverage: `gradlew :app:coverage` runs the tests of the JVM (src/test) and the tests on the
// device that adb sees (src/androidTest), prints the lines covered of each file, and fails when
// less than 90% of the lines are covered. The lines are counted over the whole module, not file by
// file. coverage.sh starts an emulator first when there is no device.
//
// Measured: every Kotlin file of src/main. generated/ is not in that folder: generate.sh makes it
// from the Rust crates, the Rust tests check it, and it is not kept in the repository.
//
// A file of src/main is left out only with the consent of the developer, with the reason here.
// scripts/verification.test.mjs guards these three values
val coverageFloor = 90
val coverageMeasured = "src/main/kotlin"
val coverageExcluded = listOf<String>()

val jacocoTool = "0.8.16"

jacoco { toolVersion = jacocoTool }

// Only `gradlew :app:coverage` counts the lines that run: the counting makes the app slower, and
// the debug build is also the one that debug.sh installs
val measuresCoverage =
    gradle.startParameter.taskNames.any { it.substringAfterLast(':') == "coverage" }

android {
    namespace = "com.sosuisha.petajournal"
    // Compose UI 1.13 needs the APIs of 37.1
    compileSdk { version = release(37) { minorApiLevel = 1 } }

    defaultConfig {
        applicationId = "com.sosuisha.petajournal"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        // The core is about 70MB for each kind of device. `-Pabi=x86_64` packs only one, for
        // an emulator with little room
        (findProperty("abi") as String?)?.let { ndk.abiFilters += it }
        manifestPlaceholders["dropboxKey"] = dropboxKey
        buildConfigField("String", "DROPBOX_KEY", "\"$dropboxKey\"")
        buildConfigField("String", "DROPBOX_FOLDER", "\"$dropboxFolder\"")
    }

    // What generate.ps1 makes from the Rust crates: the types of the border, the class that
    // calls the core, and the core as a library
    sourceSets["main"].kotlin.directories.add("../generated/kotlin")
    sourceSets["main"].jniLibs.directories.add("../generated/jniLibs")

    buildTypes {
        getByName("debug") {
            enableUnitTestCoverage = measuresCoverage
            enableAndroidTestCoverage = measuresCoverage
        }
    }
    testCoverage { jacocoVersion = jacocoTool }

    buildFeatures {
        buildConfig = true
        compose = true
    }
}

val composeUi = "1.13.0-rc01"

dependencies {
    implementation(platform("androidx.compose:compose-bom:2026.09.00"))
    // Compose UI 1.12 crashes while a long page is scrolled fast (Google Issue Tracker 549552303).
    // Only 1.13 has the full fix, and it has no stable release yet. Go back to the BOM alone
    // when a BOM brings 1.13
    implementation("androidx.compose.ui:ui:$composeUi")
    implementation("androidx.compose.foundation:foundation:$composeUi")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("com.dropbox.core:dropbox-core-sdk:8.0.2")
    implementation("com.dropbox.core:dropbox-android-sdk:8.0.2")

    testImplementation("junit:junit:4.13.2")
    // The common tests read their examples as JSON. The org.json of android.jar is a stub on the
    // JVM
    testImplementation("org.json:json:20260814")

    androidTestImplementation(platform("androidx.compose:compose-bom:2026.09.00"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4:$composeUi")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    debugImplementation("androidx.compose.ui:ui-test-manifest:$composeUi")
}

// The mutation tests: `gradlew :app:pitestDebug`. PIT runs only the tests in src/test, on the
// JVM. The tests of the screen run on a device, so PIT cannot see them, and the classes it
// mutates are only the ones that src/test tests. Why not `telling` of ItemEditor.kt: PIT takes
// whole classes, and the rest of that file is the screen
pitest {
    targetClasses.set(
        listOf(
            "com.sosuisha.petajournal.dropbox.DropboxFailuresKt",
            "com.sosuisha.petajournal.screen.PadKt",
            "com.sosuisha.petajournal.shell.AppliedKt",
            "com.sosuisha.petajournal.shell.PastedImage*",
        ),
    )
    targetTests.set(listOf("com.sosuisha.petajournal.*Test"))
    outputFormats.set(listOf("HTML", "XML"))
    timestampedReports.set(false)
}

tasks.register<JacocoReport>("coverage") {
    group = "verification"
    description =
        "Runs every test, and fails when less than $coverageFloor% of the lines are covered."
    dependsOn("testDebugUnitTest", "connectedDebugAndroidTest")
    classDirectories.from(
        tasks.named<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>("compileDebugKotlin").flatMap {
            it.destinationDirectory
        },
    )
    sourceDirectories.from(coverageMeasured)
    val unitTests = layout.buildDirectory.dir("outputs/unit_test_code_coverage")
    val deviceTests = layout.buildDirectory.dir("outputs/code_coverage")
    executionData.from(
        fileTree(unitTests) { include("**/*.exec") },
        fileTree(deviceTests) { include("**/*.ec") },
    )
    val report = layout.buildDirectory.file("reports/coverage/coverage.xml")
    reports {
        xml.required.set(true)
        xml.outputLocation.set(report)
        html.required.set(true)
        html.outputLocation.set(layout.buildDirectory.dir("reports/coverage/html"))
        csv.required.set(false)
    }

    val floor = coverageFloor
    val excluded = coverageExcluded
    val measured = layout.projectDirectory.dir(coverageMeasured)
    doLast {
        // Without the counts of one kind of test, the number would be of the other kind alone
        for (counts in listOf(unitTests, deviceTests)) {
            if (counts.get().asFileTree.isEmpty)
                throw GradleException("No test wrote ${counts.get()}")
        }
        // The report has no DTD beside it, and the parser would stop at its name
        val parser = javax.xml.parsers.DocumentBuilderFactory.newInstance()
        parser.setFeature("http://apache.org/xml/features/nonvalidating/load-external-dtd", false)
        val files =
            parser
                .newDocumentBuilder()
                .parse(report.get().asFile)
                .getElementsByTagName("sourcefile")
        var covered = 0
        var lines = 0
        for (file in (0 until files.length).map { files.item(it) as org.w3c.dom.Element }) {
            val folder = (file.parentNode as org.w3c.dom.Element).getAttribute("name")
            val path = "$folder/${file.getAttribute("name")}"
            // The classes of generated/ are in the report too: one compilation makes them all
            if (path in excluded || !measured.file(path).asFile.exists()) continue
            val counters = file.childNodes
            // A file of interfaces alone has no line that runs, and no counter of lines
            val line =
                (0 until counters.length)
                    .map { counters.item(it) }
                    .filterIsInstance<org.w3c.dom.Element>()
                    .firstOrNull { it.getAttribute("type") == "LINE" } ?: continue
            val fileCovered = line.getAttribute("covered").toInt()
            val fileLines = fileCovered + line.getAttribute("missed").toInt()
            println(
                "%6.1f%% %5d/%-5d %s"
                    .format(100.0 * fileCovered / fileLines, fileCovered, fileLines, path),
            )
            covered += fileCovered
            lines += fileLines
        }
        if (lines == 0) throw GradleException("The report has no line of $measured")
        val percent = 100.0 * covered / lines
        println("Lines covered: %.2f%% (%d/%d), floor %d%%".format(percent, covered, lines, floor))
        if (percent < floor) throw GradleException("Less than $floor% of the lines are covered")
    }
}
