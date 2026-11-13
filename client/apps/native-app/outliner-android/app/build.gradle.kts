plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

// The App key of the Dropbox app and the Markdown folder in Dropbox, such as `/PetaJournal`.
// They come from outside and are not kept in the repository (ADR 030):
// `gradlew -PdropboxKey=... -PdropboxFolder=... :app:assembleDebug`, or the same two names in
// the gradle.properties of the user. Without a key, the app is built without the sync
val dropboxKey = (findProperty("dropboxKey") as String?).orEmpty()
val dropboxFolder = (findProperty("dropboxFolder") as String?).orEmpty()

android {
    namespace = "net.sosuisen.petajournal"
    compileSdk = 37

    defaultConfig {
        applicationId = "net.sosuisen.petajournal"
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

    buildFeatures {
        buildConfig = true
        compose = true
    }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2026.09.00"))
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("com.dropbox.core:dropbox-core-sdk:8.0.2")
    implementation("com.dropbox.core:dropbox-android-sdk:8.0.2")

    testImplementation("junit:junit:4.13.2")

    androidTestImplementation(platform("androidx.compose:compose-bom:2026.09.00"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
