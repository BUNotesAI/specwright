plugins {
    id("com.android.application")
}

android {
    namespace = "com.example"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.example.androidmini"
        minSdk = 23
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
    }

    sourceSets {
        getByName("main") {
            manifest.srcFile("../AndroidManifest.xml")
        }
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}
