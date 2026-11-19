package app.witness.feature.home

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle

object AndroidPermissions {
    const val CAMERA = Manifest.permission.CAMERA
    const val FINE = Manifest.permission.ACCESS_FINE_LOCATION
    const val COARSE = Manifest.permission.ACCESS_COARSE_LOCATION
    val requested: Array<String> = arrayOf(CAMERA, FINE, COARSE)

    fun probe(context: Context, activity: Activity?, permission: String): PermissionProbe = PermissionProbe(
        granted = ContextCompat.checkSelfPermission(context, permission) == PackageManager.PERMISSION_GRANTED,
        rationale = activity?.shouldShowRequestPermissionRationale(permission) == true,
    )

    fun settingsIntent(context: Context): Intent =
        Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
}

@Composable
fun PermissionsRoute(viewModel: PermissionsViewModel, navigator: HomeNavigator) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val activity = LocalActivity.current
    fun probe(p: String) = AndroidPermissions.probe(context, activity, p)
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        if (viewModel.onResult(probe(AndroidPermissions.CAMERA), probe(AndroidPermissions.FINE))) navigator.push(HomeKey.DeviceSetup)
    }
    LifecycleResumeEffect(Unit) {
        viewModel.sync(probe(AndroidPermissions.CAMERA), probe(AndroidPermissions.FINE))
        onPauseOrDispose {}
    }
    PermissionsScreen(
        state,
        onBack = navigator.back,
        onContinue = {
            when (state.action) {
                PermissionsAction.Proceed -> navigator.push(HomeKey.DeviceSetup)
                PermissionsAction.Request -> launcher.launch(AndroidPermissions.requested)
                PermissionsAction.OpenSettings -> context.startActivity(AndroidPermissions.settingsIntent(context))
            }
        },
    )
}
