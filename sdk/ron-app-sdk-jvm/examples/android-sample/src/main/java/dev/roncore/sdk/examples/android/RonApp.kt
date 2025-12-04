package dev.roncore.sdk.examples.android;

import android.app.Application;
import android.util.Log;

import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonException;

public class RonApp extends Application {

    private RonClient ronClient;

    public RonClient getRonClient() {
        return ronClient;
    }

    @Override
    public void onCreate() {
        super.onCreate();

        String baseUrl = BuildConfig.RON_SDK_GATEWAY_ADDR;
        Log.i("RonApp", "RON Android sample starting; base URL = " + baseUrl);

        try {
            ronClient = RonClient.builder()
                    .baseUrl(baseUrl)
                    .build();
        } catch (RonException ex) {
            Log.e("RonApp", "Failed to build RonClient", ex);
            throw new RuntimeException("Failed to initialize RonClient", ex);
        }
    }
}
