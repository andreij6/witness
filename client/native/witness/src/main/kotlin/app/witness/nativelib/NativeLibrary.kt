package app.witness.nativelib

object NativeLibrary {
    const val NAME = "witness_android"
    val supportedAbis: List<String> = listOf("arm64-v8a", "x86_64")

    fun supports(abi: String): Boolean = abi in supportedAbis
}
