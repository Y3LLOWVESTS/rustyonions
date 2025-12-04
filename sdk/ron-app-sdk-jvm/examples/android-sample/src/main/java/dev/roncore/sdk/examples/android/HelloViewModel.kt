package dev.roncore.sdk.examples.android;

import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonProblem;

public class HelloViewModel {

    private final RonClient ronClient;

    public HelloViewModel(RonClient ronClient) {
        this.ronClient = ronClient;
    }

    public String load() {
        try {
            AppResponse<String> response = ronClient.get("/ping", String.class);

            if (response.ok()) {
                String data = response.getData();
                return data != null ? data : "ok";
            } else {
                RonProblem problem = response.getProblem();
                return "Error from RON: " + (problem != null ? problem.toString() : "unknown");
            }
        } catch (Exception ex) {
            String message = ex.getMessage();
            return "Exception: " + (message != null ? message : "unknown");
        }
    }
}
