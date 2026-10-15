package com.petajournal.server.domain;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class ItemBodyTest {

    @Test
    void openTaskPrefixIsStrippedAndMarksATask() {
        ItemBody body = ItemBody.parse("- [ ] Buy milk");
        assertEquals("Buy milk", body.text());
        assertTrue(body.task());
    }

    @Test
    void doneTaskPrefixIsStrippedToo() {
        ItemBody body = ItemBody.parse("- [x] Buy milk");
        assertEquals("Buy milk", body.text());
        assertTrue(body.task());
    }

    @Test
    void plainTextIsKeptAsIs() {
        ItemBody body = ItemBody.parse("- [ ]no space after the box");
        assertEquals("- [ ]no space after the box", body.text());
        assertFalse(body.task());
    }

    @Test
    void nullBodyIsEmpty() {
        ItemBody body = ItemBody.parse(null);
        assertEquals("", body.text());
        assertFalse(body.task());
        assertEquals(List.of(), body.textTypes());
    }

    @Test
    void typesComeFromTheFirstCharacterOfTheText() {
        assertEquals(List.of("heading"), ItemBody.parse("# Title").textTypes());
        assertEquals(List.of("code"), ItemBody.parse("  ```java\nx\n```").textTypes());
        assertEquals(List.of("blockquote"), ItemBody.parse("> quote").textTypes());
        assertEquals(List.of(), ItemBody.parse("text").textTypes());
    }

    @Test
    void taskPrefixDoesNotHideTheTextTypes() {
        ItemBody body = ItemBody.parse("- [ ] # Heading task");
        assertTrue(body.task());
        assertEquals(List.of("heading"), body.textTypes());
    }
}
