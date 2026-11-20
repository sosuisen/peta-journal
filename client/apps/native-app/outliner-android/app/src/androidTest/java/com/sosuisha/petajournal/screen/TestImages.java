package com.sosuisha.petajournal.screen;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;

import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.OutputStream;

/**
 * The images of the tests, lent by a {@code content://} address as another app lends the image
 * it put on the clipboard. Every image has the content {@link #DRAWING}.
 *
 * <p>Why Java: the system runs this in a process of the test package, which has no Kotlin
 * library. Why no file: that process may not read the files of the app under test.
 */
public final class TestImages extends ContentProvider {
    private static final String AUTHORITY = "com.sosuisha.petajournal.test.images";

    /** The content of every image. */
    public static final byte[] DRAWING = {(byte) 0x89, 'P', 'N', 'G'};

    /** The address of an image named {@code name}. */
    public static Uri address(String name) {
        return Uri.parse("content://" + AUTHORITY + "/" + name);
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public String getType(Uri uri) {
        return "image/png";
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        try {
            ParcelFileDescriptor[] pipe = ParcelFileDescriptor.createPipe();
            try (OutputStream out = new ParcelFileDescriptor.AutoCloseOutputStream(pipe[1])) {
                out.write(DRAWING);
            }
            return pipe[0];
        } catch (IOException e) {
            throw new FileNotFoundException(e.getMessage());
        }
    }

    @Override
    public Cursor query(Uri uri, String[] projection, String selection, String[] args, String order) {
        MatrixCursor names = new MatrixCursor(new String[] {OpenableColumns.DISPLAY_NAME});
        names.addRow(new Object[] {uri.getLastPathSegment()});
        return names;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        return null;
    }

    @Override
    public int delete(Uri uri, String selection, String[] args) {
        return 0;
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] args) {
        return 0;
    }
}
