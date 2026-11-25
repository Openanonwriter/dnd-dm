// Thin Android shell around the Rust library. There is no Java/Kotlin code:
// Android's NativeActivity loads libdnd_player.so (built by cargo-ndk into
// src/main/jniLibs) and the whole UI is egui.
plugins {
    id("com.android.application")
}

android {
    namespace = "dev.dnddm.player"
    compileSdk = 34
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "dev.dnddm.player"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1"
        ndk { abiFilters += listOf("arm64-v8a") }
    }

    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")

    buildTypes {
        release {
            isMinifyEnabled = false
            // sideload-friendly: sign releases with the debug key unless you add your own
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
