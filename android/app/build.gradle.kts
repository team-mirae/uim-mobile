import java.io.File

plugins {
    id("com.android.application")
}

android {
    namespace = "me.sjun.uim"
    compileSdk = 36
    ndkVersion = "30.0.16248370"

    defaultConfig {
        applicationId = "me.sjun.uim"
        minSdk = 29
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "4.1.2"
        }
    }
}

// The engine (../engine) as a static library per ABI, linked into the JNI wrapper by CMake.
val engineDir = rootProject.file("../engine")
val koreanDir = rootProject.file("../../jis-korean/engine")
val cargo = File(System.getProperty("user.home"), ".cargo/bin/cargo").takeIf { it.exists() }?.path ?: "cargo"
val rustTargets = listOf("aarch64-linux-android", "armv7-linux-androideabi", "x86_64-linux-android")

val buildEngine by tasks.registering {
    group = "build"
    description = "Builds the Rust engine as a static library for every Android ABI."
    inputs.dir(engineDir.resolve("src"))
    inputs.file(engineDir.resolve("Cargo.toml"))
    inputs.dir(koreanDir.resolve("src"))
    outputs.files(rustTargets.map { engineDir.resolve("target/$it/release/libuim.a") })
    doLast {
        for (target in rustTargets) {
            providers.exec {
                workingDir = engineDir
                commandLine(cargo, "rustc", "--release", "--lib", "--crate-type", "staticlib", "--target", target)
            }.result.get().assertNormalExitValue()
        }
    }
}

// EN predictive's word list and bigrams, and JA's conversion data with its licence (engine/data),
// as assets, read at start.
val dataDir = layout.buildDirectory.dir("generated/engine-data").get().asFile
val copyData by tasks.registering(Copy::class) {
    from(engineDir.resolve("data")) { include("en-words.txt", "en-bigrams.txt", "en-acronyms.txt", "LICENSE-scowl.txt", "kanji.bin", "LICENSE-mozc.txt") }
    into(dataDir)
}
android.sourceSets.getByName("main").assets.srcDir(dataDir)

tasks.named("preBuild") { dependsOn(buildEngine, copyData) }
