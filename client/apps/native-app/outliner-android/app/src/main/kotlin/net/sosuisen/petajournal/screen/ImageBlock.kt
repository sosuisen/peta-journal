package net.sosuisen.petajournal.screen

import android.graphics.BitmapFactory
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.rememberTransformableState
import androidx.compose.foundation.gestures.transformable
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What is known of the file of an image. */
private sealed interface Picture {
    /** Not read yet. Nothing is drawn, so that the alt does not flash before the image. */
    data object Reading : Picture

    data object Missing : Picture

    /** @param width the width of the image in its file, which [bitmap] may be smaller than. */
    data class Here(val bitmap: ImageBitmap, val width: Int) : Picture
}

/**
 * An image, as wide as the page asks, or else as wide as it is in its file. A pixel of the
 * page is one `dp`, as in a browser. It is never wider than the room it has. An image whose
 * file is not here, or cannot be read, shows what stands for it.
 */
@Composable
internal fun ImageBlock(image: ShownBlock.Image, file: (String) -> File?) {
    val roomPx = LocalWindowInfo.current.containerSize.width
    val picture by produceState<Picture>(Picture.Reading, image.path) {
        value = withContext(Dispatchers.IO) { file(image.path)?.let { read(it, roomPx) } ?: Picture.Missing }
    }
    when (val known = picture) {
        Picture.Reading -> Unit
        Picture.Missing -> Text(image.alt.ifEmpty { image.path }, style = MaterialTheme.typography.bodyLarge)
        is Picture.Here -> {
            var large by remember { mutableStateOf(false) }
            val shape = known.bitmap.width.toFloat() / known.bitmap.height
            // The side that the page does not ask for follows the shape of the image
            val width = image.width?.toFloat() ?: image.height?.let { it * shape } ?: known.width.toFloat()
            val height = image.height?.toFloat() ?: (width / shape)
            Image(
                known.bitmap,
                contentDescription = image.alt,
                // A page that asks for both sides may ask for another shape, as in a browser
                contentScale = ContentScale.FillBounds,
                modifier = Modifier
                    .padding(vertical = 4.dp)
                    .width(width.dp)
                    .aspectRatio(width / height)
                    .clickable { large = true },
            )
            if (large) LargeImage(image, known.bitmap, file) { large = false }
        }
    }
}

/** How many times the width of the screen an image is read for its large view, at least. */
private const val LARGE_WIDTHS = 2

/** How far two fingers make the large view larger. */
private const val MOST_ZOOM = 8f

/**
 * An image over the whole screen, to read what is small in it. Two fingers make it larger and
 * move it. A touch, or the back key, closes it.
 *
 * The image is read again with more of its pixels. Until then, [small] is shown.
 */
@Composable
private fun LargeImage(image: ShownBlock.Image, small: ImageBitmap, file: (String) -> File?, onClose: () -> Unit) {
    val roomPx = LocalWindowInfo.current.containerSize.width
    val sharp by produceState<ImageBitmap?>(null, image.path) {
        value = withContext(Dispatchers.IO) { file(image.path)?.let { read(it, roomPx * LARGE_WIDTHS) }?.bitmap }
    }
    var zoom by remember { mutableFloatStateOf(1f) }
    var moved by remember { mutableStateOf(Offset.Zero) }
    var room by remember { mutableStateOf(IntSize.Zero) }
    val fingers = rememberTransformableState { zoomBy, panBy, _ ->
        zoom = (zoom * zoomBy).coerceIn(1f, MOST_ZOOM)
        // The image never leaves the screen: it moves only as far as it is larger
        val farX = room.width * (zoom - 1) / 2
        val farY = room.height * (zoom - 1) / 2
        moved = Offset((moved.x + panBy.x).coerceIn(-farX, farX), (moved.y + panBy.y).coerceIn(-farY, farY))
    }
    Dialog(onClose, DialogProperties(usePlatformDefaultWidth = false)) {
        Image(
            sharp ?: small,
            contentDescription = image.alt,
            contentScale = ContentScale.Fit,
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black)
                .onSizeChanged { room = it }
                .clickable(interactionSource = null, indication = null, onClick = onClose)
                .transformable(fingers)
                .graphicsLayer {
                    scaleX = zoom
                    scaleY = zoom
                    translationX = moved.x
                    translationY = moved.y
                },
        )
    }
}

/**
 * Reads the image of the file. A large image is read smaller, by whole halves, down to no
 * less than [roomPx] wide: a photo at its full size takes the memory of many screens.
 */
private fun read(file: File, roomPx: Int): Picture.Here? {
    val size = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeFile(file.path, size)
    val halves = generateSequence(1) { it * 2 }.takeWhile { size.outWidth / it >= roomPx }.lastOrNull() ?: 1
    val bitmap = BitmapFactory.decodeFile(file.path, BitmapFactory.Options().apply { inSampleSize = halves })
    return bitmap?.let { Picture.Here(it.asImageBitmap(), size.outWidth) }
}
