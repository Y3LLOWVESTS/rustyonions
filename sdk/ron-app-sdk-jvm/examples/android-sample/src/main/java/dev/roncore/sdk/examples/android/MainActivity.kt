package dev.roncore.sdk.examples.android;

import android.os.Bundle;
import android.widget.Button;
import android.widget.TextView;

import androidx.appcompat.app.AppCompatActivity;

public class MainActivity extends AppCompatActivity {

    private HelloViewModel viewModel;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        RonApp app = (RonApp) getApplication();
        viewModel = new HelloViewModel(app.getRonClient());

        TextView textView = findViewById(R.id.helloText);
        Button button = findViewById(R.id.pingButton);

        textView.setText("Tap the button to ping RON…");

        button.setOnClickListener(v -> {
            textView.setText("Pinging RON…");

            new Thread(() -> {
                String result = viewModel.load();
                runOnUiThread(() -> textView.setText(result));
            }).start();
        });
    }
}
