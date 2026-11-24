// The Android plugin brings Kotlin 2.2.0. The Dropbox SDK 8.0.2 needs Kotlin 2.4 (ADR 030), so
// the Kotlin plugin is named here.
buildscript {
    dependencies {
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.21")
    }
}

plugins {
    id("com.android.application") version "9.5.1" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.4.21" apply false
    id("com.ncorti.ktfmt.gradle") version "0.28.0" apply false
    id("pl.droidsonroids.pitest") version "0.3.0" apply false
}
