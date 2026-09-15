package io.warpnine.markdownja;

/** Stateless, thread-safe Japanese-aware Markdown formatting (UTF-16 in/out). */
public final class MarkdownFormatter {
    static { System.loadLibrary("dprint_markdown_ja_formatter"); }

    private MarkdownFormatter() {}

    public enum TextWrap {
        NEVER("never"), MAINTAIN("maintain"), ALWAYS("always");
        final String value;
        TextWrap(String value) { this.value = value; }
    }

    public enum Marker {
        UNDERSCORES("underscores"), ASTERISKS("asterisks");
        final String value;
        Marker(String value) { this.value = value; }
    }

    /** Returns formatted text, including the original content when unchanged. */
    public static String format(String text) {
        return formatNative(text, 80, "never", "underscores", "asterisks");
    }

    /** @throws IllegalArgumentException for null, malformed UTF-16, or invalid options */
    public static String format(String text, int lineWidth, TextWrap wrap,
                                Marker emphasis, Marker strong) {
        return formatNative(text, lineWidth, wrap == null ? null : wrap.value,
                emphasis == null ? null : emphasis.value, strong == null ? null : strong.value);
    }

    private static native String formatNative(String text, int lineWidth, String wrap,
                                               String emphasis, String strong);
}
