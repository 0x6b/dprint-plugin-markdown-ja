import io.warpnine.markdownja.MarkdownFormatter;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.concurrent.*;

public final class IntegrationTest {
    static void equal(String expected, String actual) {
        if (!expected.equals(actual)) throw new AssertionError("Expected: " + expected + " Actual: " + actual);
    }

    static void invalid(Method nativeMethod, Object... args) throws Exception {
        try {
            nativeMethod.invoke(null, args);
            throw new AssertionError("Expected invalid argument exception");
        } catch (InvocationTargetException e) {
            if (!(e.getCause() instanceof IllegalArgumentException) || e.getCause().getMessage().isEmpty()) throw e;
        }
    }

    public static void main(String[] args) throws Exception {
        equal("日本語 English _text_ **bold**\n", MarkdownFormatter.format("日本語English *text* __bold__"));
        equal("alpha beta gamma\ndelta epsilon\n", MarkdownFormatter.format("alpha beta gamma delta epsilon", 16,
                MarkdownFormatter.TextWrap.ALWAYS, MarkdownFormatter.Marker.ASTERISKS, MarkdownFormatter.Marker.UNDERSCORES));
        equal("*text* __bold__\n", MarkdownFormatter.format("_text_ **bold**", 80,
                MarkdownFormatter.TextWrap.NEVER, MarkdownFormatter.Marker.ASTERISKS, MarkdownFormatter.Marker.UNDERSCORES));
        String unchanged = "<!-- dprint-ignore-file -->\n日本語\uD83D\uDE80\u0000e\u0301";
        equal(unchanged, MarkdownFormatter.format(unchanged));
        equal("```md\n*raw*日本語 English\n```\n", MarkdownFormatter.format("~~~md\n*raw*日本語English  \n~~~\n"));
        equal("- _raw_\n\n  ```js\n  x\n  ```\n", MarkdownFormatter.format("- *raw*\n\n  ```js\n\n    x  \n\n  ```"));
        Method nativeMethod = MarkdownFormatter.class.getDeclaredMethod("formatNative", String.class, int.class, String.class, String.class, String.class);
        nativeMethod.setAccessible(true);
        invalid(nativeMethod, null, 80, "never", "underscores", "asterisks");
        invalid(nativeMethod, "\uD800", 80, "never", "underscores", "asterisks");
        invalid(nativeMethod, "x", -1, "never", "underscores", "asterisks");
        invalid(nativeMethod, "x", 10001, "never", "underscores", "asterisks");
        invalid(nativeMethod, "x", 80, "bad", "underscores", "asterisks");
        invalid(nativeMethod, "x", 80, "never", "bad", "asterisks");
        invalid(nativeMethod, "x", 80, "never", "underscores", "bad");
        invalid(nativeMethod, "x", 80, null, "underscores", "asterisks");
        ExecutorService pool = Executors.newFixedThreadPool(4);
        try {
            java.util.List<Future<?>> jobs = new java.util.ArrayList<>();
            for (int t = 0; t < 4; t++) jobs.add(pool.submit(() -> {
                for (int i = 0; i < 1000; i++) equal("日本語 English\n", MarkdownFormatter.format("日本語English"));
            }));
            for (Future<?> job : jobs) job.get();
        } finally { pool.shutdown(); }
        System.out.println("JNI integration passed: Unicode, fences, options, errors, unchanged text, 4000 concurrent/repeated calls");
    }
}
