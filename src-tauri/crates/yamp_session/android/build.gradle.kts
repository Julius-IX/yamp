import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

// The Discord Social SDK is downloaded by hand and is not in the repository. Without it the
// presence is left out, and nothing here needs a native toolchain. See the readme of yamp-discord.
val discordSdk = file("libs/discord_partner_sdk.aar")

android {
    namespace = "com.salad.yamp.session"
    compileSdk = 37

    defaultConfig {
        minSdk = 24
        if (discordSdk.exists()) {
            externalNativeBuild { cmake { arguments("-DANDROID_STL=c++_shared") } }
        }
    }
    if (discordSdk.exists()) {
        buildFeatures { prefab = true }
        externalNativeBuild { cmake { path = file("src/main/cpp/CMakeLists.txt") } }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.9.0")
    implementation(project(":tauri-android"))
    // compile only, since a library cannot carry a local aar. The app brings it along, see its gradle file
    if (discordSdk.exists()) compileOnly(files(discordSdk))
}
