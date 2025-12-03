mymac@RustyOnionsDevNet ron-app-sdk-jvm % cd /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm

# Check Gradle sees subprojects
./gradlew projects

# Run the full test script with extra logging for the first failure
tools/ci/run-tests.sh

zsh: command not found: #

> Task :projects

Projects:

------------------------------------------------------------
Root project 'ron-app-sdk-jvm'
------------------------------------------------------------

Root project 'ron-app-sdk-jvm'
No sub-projects

To see a list of the tasks of a project, run gradlew <project-path>:tasks
For example, try running gradlew :tasks

BUILD SUCCESSFUL in 1s
1 actionable task: 1 executed
zsh: command not found: #
[ron-app-sdk-jvm] running JVM SDK tests...
  ROOT=/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm

[ron-app-sdk-jvm] ::core:test

FAILURE: Build failed with an exception.

* What went wrong:
Cannot locate tasks that match ':core:test' as project 'core' not found in root project 'ron-app-sdk-jvm'.

* Try:
> Run gradlew projects to get a list of available projects.
> For more on name expansion, please refer to https://docs.gradle.org/8.10.2/userguide/command_line_interface.html#sec:name_abbreviation in the Gradle documentation.
> Run with --stacktrace option to get the stack trace.
> Run with --info or --debug option to get more log output.
> Run with --scan to get full insights.
> Get more help at https://help.gradle.org.

BUILD FAILED in 1s
mymac@RustyOnionsDevNet ron-app-sdk-jvm % ./gradlew :core:test --stacktrace --info 

Initialized native services in: /Users/mymac/.gradle/native
Initialized jansi services in: /Users/mymac/.gradle/native
Received JVM installation metadata from '/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home': {JAVA_HOME=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home, JAVA_VERSION=17.0.17, JAVA_VENDOR=Eclipse Adoptium, RUNTIME_NAME=OpenJDK Runtime Environment, RUNTIME_VERSION=17.0.17+10, VM_NAME=OpenJDK 64-Bit Server VM, VM_VERSION=17.0.17+10, VM_VENDOR=Eclipse Adoptium, OS_ARCH=x86_64}
Found daemon DaemonInfo{pid=1493, address=[f6b72d31-b58a-4fe1-8f9d-2c84fba69bf3 port:49317, addresses:[/127.0.0.1]], state=Idle, lastBusy=1764787157477, context=DefaultDaemonContext[uid=1bb8a592-74c7-49fd-9667-9fcb83bcebf2,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1493,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=utf8,-Duser.country=US,-Duser.language=en,-Duser.variant]} however its context does not match the desired criteria.
At least one daemon option is different.
Wanted: DaemonRequestContext{jvmCriteria=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home (no JDK specified, using current Java home), daemonOpts=[--add-opens=java.base/java.util=ALL-UNNAMED, --add-opens=java.base/java.lang=ALL-UNNAMED, --add-opens=java.base/java.lang.invoke=ALL-UNNAMED, --add-opens=java.prefs/java.util.prefs=ALL-UNNAMED, --add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED, --add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED, --add-opens=java.base/java.nio.charset=ALL-UNNAMED, --add-opens=java.base/java.net=ALL-UNNAMED, --add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED, -XX:MaxMetaspaceSize=384m, -XX:+HeapDumpOnOutOfMemoryError, -Xms256m, -Xmx512m, -Dfile.encoding=UTF-8, -Duser.country=US, -Duser.language=en, -Duser.variant], applyInstrumentationAgent=true, nativeServicesMode=ENABLED, priority=NORMAL}
Actual: DefaultDaemonContext[uid=1bb8a592-74c7-49fd-9667-9fcb83bcebf2,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1493,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=utf8,-Duser.country=US,-Duser.language=en,-Duser.variant]

  Looking for a different daemon...
The client will now receive all logging from the daemon (pid: 1456). The daemon log file: /Users/mymac/.gradle/daemon/8.10.2/daemon-1456.out.log
Starting 54th build in daemon [uptime: 1 hrs 26 mins 48.932 secs, performance: 99%, GC rate: 0.00/s, heap usage: 0% of 512 MiB, non-heap usage: 37% of 384 MiB]
Using 8 worker leases.
Now considering [/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm] as hierarchies to watch
Watching the file system is configured to be enabled if available
File system watching is active
Starting Build
Settings evaluated using settings file '/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/settings.gradle.kts'.
Projects loaded. Root project using build file '/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/build.gradle.kts'.
Included projects: [root project 'ron-app-sdk-jvm']

> Configure project :
Evaluating root project 'ron-app-sdk-jvm' using build file '/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/build.gradle.kts'.
Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with InstrumentationAnalysisTransform
Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with InstrumentationAnalysisTransform
Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with InstrumentationAnalysisTransform
Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with MergeInstrumentationAnalysisTransform
Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with InstrumentationAnalysisTransform
Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with MergeInstrumentationAnalysisTransform
Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with InstrumentationAnalysisTransform
Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with MergeInstrumentationAnalysisTransform
Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with InstrumentationAnalysisTransform
Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with MergeInstrumentationAnalysisTransform
Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with InstrumentationAnalysisTransform
Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with MergeInstrumentationAnalysisTransform
Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
All projects evaluated.

FAILURE: Build failed with an exception.

* What went wrong:
Cannot locate tasks that match ':core:test' as project 'core' not found in root project 'ron-app-sdk-jvm'.

* Try:
> Run gradlew projects to get a list of available projects.
> For more on name expansion, please refer to https://docs.gradle.org/8.10.2/userguide/command_line_interface.html#sec:name_abbreviation in the Gradle documentation.
> Run with --debug option to get more log output.
> Run with --scan to get full insights.
> Get more help at https://help.gradle.org.

* Exception is:
org.gradle.execution.ProjectSelectionException: Cannot locate tasks that match ':core:test' as project 'core' not found in root project 'ron-app-sdk-jvm'.
        at org.gradle.execution.selection.DefaultBuildTaskSelector.lambda$selectProject$2(DefaultBuildTaskSelector.java:180)
        at org.gradle.api.problems.internal.DefaultProblemReporter.throwing(DefaultProblemReporter.java:57)
        at org.gradle.execution.selection.DefaultBuildTaskSelector.selectProject(DefaultBuildTaskSelector.java:177)
        at org.gradle.execution.selection.DefaultBuildTaskSelector.resolveProject(DefaultBuildTaskSelector.java:135)
        at org.gradle.execution.selection.DefaultBuildTaskSelector.resolveTaskName(DefaultBuildTaskSelector.java:107)
        at org.gradle.execution.commandline.CommandLineTaskParser.parseTasks(CommandLineTaskParser.java:49)
        at org.gradle.execution.TaskNameResolvingBuildTaskScheduler.scheduleRequestedTasks(TaskNameResolvingBuildTaskScheduler.java:51)
        at org.gradle.execution.DefaultTasksBuildTaskScheduler.scheduleRequestedTasks(DefaultTasksBuildTaskScheduler.java:72)
        at org.gradle.initialization.DefaultTaskExecutionPreparer.lambda$scheduleRequestedTasks$0(DefaultTaskExecutionPreparer.java:46)
        at org.gradle.internal.Factories$1.create(Factories.java:31)
        at org.gradle.internal.work.DefaultWorkerLeaseService.withReplacedLocks(DefaultWorkerLeaseService.java:359)
        at org.gradle.api.internal.project.DefaultProjectStateRegistry$DefaultBuildProjectRegistry.withMutableStateOfAllProjects(DefaultProjectStateRegistry.java:238)
        at org.gradle.api.internal.project.DefaultProjectStateRegistry$DefaultBuildProjectRegistry.withMutableStateOfAllProjects(DefaultProjectStateRegistry.java:231)
        at org.gradle.initialization.DefaultTaskExecutionPreparer.scheduleRequestedTasks(DefaultTaskExecutionPreparer.java:45)
        at org.gradle.initialization.VintageBuildModelController.lambda$scheduleRequestedTasks$0(VintageBuildModelController.java:76)
        at org.gradle.internal.model.StateTransitionController.lambda$inState$1(StateTransitionController.java:99)
        at org.gradle.internal.model.StateTransitionController.lambda$inState$2(StateTransitionController.java:114)
        at org.gradle.internal.work.DefaultSynchronizer.withLock(DefaultSynchronizer.java:44)
        at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:110)
        at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:98)
        at org.gradle.initialization.VintageBuildModelController.scheduleRequestedTasks(VintageBuildModelController.java:76)
        at org.gradle.internal.build.DefaultBuildLifecycleController$DefaultWorkGraphBuilder.addRequestedTasks(DefaultBuildLifecycleController.java:404)
        at org.gradle.internal.buildtree.DefaultBuildTreeWorkPreparer.lambda$scheduleRequestedTasks$0(DefaultBuildTreeWorkPreparer.java:41)
        at org.gradle.internal.build.DefaultBuildLifecycleController.lambda$populateWorkGraph$7(DefaultBuildLifecycleController.java:189)
        at org.gradle.internal.build.DefaultBuildWorkPreparer.populateWorkGraph(DefaultBuildWorkPreparer.java:42)
        at org.gradle.internal.build.BuildOperationFiringBuildWorkPreparer$PopulateWorkGraph.populateTaskGraph(BuildOperationFiringBuildWorkPreparer.java:106)
        at org.gradle.internal.build.BuildOperationFiringBuildWorkPreparer$PopulateWorkGraph.run(BuildOperationFiringBuildWorkPreparer.java:92)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:29)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:26)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:66)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:59)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:166)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:59)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.run(DefaultBuildOperationRunner.java:47)
        at org.gradle.internal.build.BuildOperationFiringBuildWorkPreparer.populateWorkGraph(BuildOperationFiringBuildWorkPreparer.java:67)
        at org.gradle.internal.build.DefaultBuildLifecycleController.lambda$populateWorkGraph$8(DefaultBuildLifecycleController.java:189)
        at org.gradle.internal.model.StateTransitionController.lambda$inState$1(StateTransitionController.java:99)
        at org.gradle.internal.model.StateTransitionController.lambda$inState$2(StateTransitionController.java:114)
        at org.gradle.internal.work.DefaultSynchronizer.withLock(DefaultSynchronizer.java:44)
        at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:110)
        at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:98)
        at org.gradle.internal.build.DefaultBuildLifecycleController.populateWorkGraph(DefaultBuildLifecycleController.java:189)
        at org.gradle.internal.build.DefaultBuildWorkGraphController$DefaultBuildWorkGraph.populateWorkGraph(DefaultBuildWorkGraphController.java:169)
        at org.gradle.composite.internal.DefaultBuildController.populateWorkGraph(DefaultBuildController.java:76)
        at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph$DefaultBuildTreeWorkGraphBuilder.withWorkGraph(DefaultIncludedBuildTaskGraph.java:153)
        at org.gradle.internal.buildtree.DefaultBuildTreeWorkPreparer.lambda$scheduleRequestedTasks$1(DefaultBuildTreeWorkPreparer.java:41)
        at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph$DefaultBuildTreeWorkGraph$1.run(DefaultIncludedBuildTaskGraph.java:209)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:29)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:26)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:66)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:59)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:166)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:59)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.run(DefaultBuildOperationRunner.java:47)
        at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph$DefaultBuildTreeWorkGraph.scheduleWork(DefaultIncludedBuildTaskGraph.java:204)
        at org.gradle.internal.buildtree.DefaultBuildTreeWorkPreparer.scheduleRequestedTasks(DefaultBuildTreeWorkPreparer.java:37)
        at org.gradle.internal.cc.impl.VintageBuildTreeWorkController$scheduleAndRunRequestedTasks$1.apply(VintageBuildTreeWorkController.kt:36)
        at org.gradle.internal.cc.impl.VintageBuildTreeWorkController$scheduleAndRunRequestedTasks$1.apply(VintageBuildTreeWorkController.kt:35)
        at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph.withNewWorkGraph(DefaultIncludedBuildTaskGraph.java:112)
        at org.gradle.internal.cc.impl.VintageBuildTreeWorkController.scheduleAndRunRequestedTasks(VintageBuildTreeWorkController.kt:35)
        at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.lambda$scheduleAndRunTasks$1(DefaultBuildTreeLifecycleController.java:77)
        at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.lambda$runBuild$4(DefaultBuildTreeLifecycleController.java:120)
        at org.gradle.internal.model.StateTransitionController.lambda$transition$6(StateTransitionController.java:169)
        at org.gradle.internal.model.StateTransitionController.doTransition(StateTransitionController.java:266)
        at org.gradle.internal.model.StateTransitionController.lambda$transition$7(StateTransitionController.java:169)
        at org.gradle.internal.work.DefaultSynchronizer.withLock(DefaultSynchronizer.java:44)
        at org.gradle.internal.model.StateTransitionController.transition(StateTransitionController.java:169)
        at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.runBuild(DefaultBuildTreeLifecycleController.java:117)
        at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.scheduleAndRunTasks(DefaultBuildTreeLifecycleController.java:77)
        at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.scheduleAndRunTasks(DefaultBuildTreeLifecycleController.java:72)
        at org.gradle.tooling.internal.provider.ExecuteBuildActionRunner.run(ExecuteBuildActionRunner.java:31)
        at org.gradle.launcher.exec.ChainingBuildActionRunner.run(ChainingBuildActionRunner.java:35)
        at org.gradle.internal.buildtree.ProblemReportingBuildActionRunner.run(ProblemReportingBuildActionRunner.java:49)
        at org.gradle.launcher.exec.BuildOutcomeReportingBuildActionRunner.run(BuildOutcomeReportingBuildActionRunner.java:65)
        at org.gradle.tooling.internal.provider.FileSystemWatchingBuildActionRunner.run(FileSystemWatchingBuildActionRunner.java:140)
        at org.gradle.launcher.exec.BuildCompletionNotifyingBuildActionRunner.run(BuildCompletionNotifyingBuildActionRunner.java:41)
        at org.gradle.launcher.exec.RootBuildLifecycleBuildActionExecutor.lambda$execute$0(RootBuildLifecycleBuildActionExecutor.java:54)
        at org.gradle.composite.internal.DefaultRootBuildState.run(DefaultRootBuildState.java:130)
        at org.gradle.launcher.exec.RootBuildLifecycleBuildActionExecutor.execute(RootBuildLifecycleBuildActionExecutor.java:54)
        at org.gradle.internal.buildtree.InitDeprecationLoggingActionExecutor.execute(InitDeprecationLoggingActionExecutor.java:62)
        at org.gradle.internal.buildtree.InitProblems.execute(InitProblems.java:36)
        at org.gradle.internal.buildtree.DefaultBuildTreeContext.execute(DefaultBuildTreeContext.java:40)
        at org.gradle.launcher.exec.BuildTreeLifecycleBuildActionExecutor.lambda$execute$0(BuildTreeLifecycleBuildActionExecutor.java:71)
        at org.gradle.internal.buildtree.BuildTreeState.run(BuildTreeState.java:60)
        at org.gradle.launcher.exec.BuildTreeLifecycleBuildActionExecutor.execute(BuildTreeLifecycleBuildActionExecutor.java:71)
        at org.gradle.launcher.exec.RunAsBuildOperationBuildActionExecutor$3.call(RunAsBuildOperationBuildActionExecutor.java:61)
        at org.gradle.launcher.exec.RunAsBuildOperationBuildActionExecutor$3.call(RunAsBuildOperationBuildActionExecutor.java:57)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$CallableBuildOperationWorker.execute(DefaultBuildOperationRunner.java:209)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$CallableBuildOperationWorker.execute(DefaultBuildOperationRunner.java:204)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:66)
        at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:59)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:166)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:59)
        at org.gradle.internal.operations.DefaultBuildOperationRunner.call(DefaultBuildOperationRunner.java:53)
        at org.gradle.launcher.exec.RunAsBuildOperationBuildActionExecutor.execute(RunAsBuildOperationBuildActionExecutor.java:57)
        at org.gradle.launcher.exec.RunAsWorkerThreadBuildActionExecutor.lambda$execute$0(RunAsWorkerThreadBuildActionExecutor.java:36)
        at org.gradle.internal.work.DefaultWorkerLeaseService.withLocks(DefaultWorkerLeaseService.java:263)
        at org.gradle.internal.work.DefaultWorkerLeaseService.runAsWorkerThread(DefaultWorkerLeaseService.java:127)
        at org.gradle.launcher.exec.RunAsWorkerThreadBuildActionExecutor.execute(RunAsWorkerThreadBuildActionExecutor.java:36)
        at org.gradle.tooling.internal.provider.continuous.ContinuousBuildActionExecutor.execute(ContinuousBuildActionExecutor.java:110)
        at org.gradle.tooling.internal.provider.SubscribableBuildActionExecutor.execute(SubscribableBuildActionExecutor.java:64)
        at org.gradle.internal.session.DefaultBuildSessionContext.execute(DefaultBuildSessionContext.java:46)
        at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor$ActionImpl.apply(BuildSessionLifecycleBuildActionExecutor.java:92)
        at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor$ActionImpl.apply(BuildSessionLifecycleBuildActionExecutor.java:80)
        at org.gradle.internal.session.BuildSessionState.run(BuildSessionState.java:71)
        at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor.execute(BuildSessionLifecycleBuildActionExecutor.java:62)
        at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor.execute(BuildSessionLifecycleBuildActionExecutor.java:41)
        at org.gradle.internal.buildprocess.execution.StartParamsValidatingActionExecutor.execute(StartParamsValidatingActionExecutor.java:64)
        at org.gradle.internal.buildprocess.execution.StartParamsValidatingActionExecutor.execute(StartParamsValidatingActionExecutor.java:32)
        at org.gradle.internal.buildprocess.execution.SessionFailureReportingActionExecutor.execute(SessionFailureReportingActionExecutor.java:51)
        at org.gradle.internal.buildprocess.execution.SessionFailureReportingActionExecutor.execute(SessionFailureReportingActionExecutor.java:39)
        at org.gradle.internal.buildprocess.execution.SetupLoggingActionExecutor.execute(SetupLoggingActionExecutor.java:47)
        at org.gradle.internal.buildprocess.execution.SetupLoggingActionExecutor.execute(SetupLoggingActionExecutor.java:31)
        at org.gradle.launcher.daemon.server.exec.ExecuteBuild.doBuild(ExecuteBuild.java:70)
        at org.gradle.launcher.daemon.server.exec.BuildCommandOnly.execute(BuildCommandOnly.java:37)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.WatchForDisconnection.execute(WatchForDisconnection.java:39)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.ResetDeprecationLogger.execute(ResetDeprecationLogger.java:29)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.RequestStopIfSingleUsedDaemon.execute(RequestStopIfSingleUsedDaemon.java:35)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.ForwardClientInput.lambda$execute$0(ForwardClientInput.java:40)
        at org.gradle.internal.daemon.clientinput.ClientInputForwarder.forwardInput(ClientInputForwarder.java:80)
        at org.gradle.launcher.daemon.server.exec.ForwardClientInput.execute(ForwardClientInput.java:37)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.LogAndCheckHealth.execute(LogAndCheckHealth.java:64)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.LogToClient.doBuild(LogToClient.java:63)
        at org.gradle.launcher.daemon.server.exec.BuildCommandOnly.execute(BuildCommandOnly.java:37)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.EstablishBuildEnvironment.doBuild(EstablishBuildEnvironment.java:84)
        at org.gradle.launcher.daemon.server.exec.BuildCommandOnly.execute(BuildCommandOnly.java:37)
        at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
        at org.gradle.launcher.daemon.server.exec.StartBuildOrRespondWithBusy$1.run(StartBuildOrRespondWithBusy.java:52)
        at org.gradle.launcher.daemon.server.DaemonStateCoordinator.lambda$runCommand$0(DaemonStateCoordinator.java:321)
        at org.gradle.internal.concurrent.ExecutorPolicy$CatchAndRecordFailures.onExecute(ExecutorPolicy.java:64)
        at org.gradle.internal.concurrent.AbstractManagedExecutor$1.run(AbstractManagedExecutor.java:48)


BUILD FAILED in 1s
Watched directory hierarchies: [/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm]
mymac@RustyOnionsDevNet ron-app-sdk-jvm % ./gradlew :core:test --stacktrace --debug

2025-12-03T12:40:05.914-0600 [INFO] [org.gradle.internal.nativeintegration.services.NativeServices] Initialized native services in: /Users/mymac/.gradle/native
2025-12-03T12:40:06.000-0600 [INFO] [org.gradle.internal.nativeintegration.services.NativeServices] Initialized jansi services in: /Users/mymac/.gradle/native
2025-12-03T12:40:06.044-0600 [LIFECYCLE] [org.gradle.launcher.cli.DebugLoggerWarningAction] 
#############################################################################
   WARNING WARNING WARNING WARNING WARNING WARNING WARNING WARNING WARNING

   Debug level logging will leak security sensitive information!

   For more details, please refer to https://docs.gradle.org/8.10.2/userguide/logging.html#sec:debug_security in the Gradle documentation.
#############################################################################

2025-12-03T12:40:06.211-0600 [INFO] [org.gradle.internal.jvm.inspection.DefaultJvmMetadataDetector] Received JVM installation metadata from '/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home': {JAVA_HOME=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home, JAVA_VERSION=17.0.17, JAVA_VENDOR=Eclipse Adoptium, RUNTIME_NAME=OpenJDK Runtime Environment, RUNTIME_VERSION=17.0.17+10, VM_NAME=OpenJDK 64-Bit Server VM, VM_VERSION=17.0.17+10, VM_VENDOR=Eclipse Adoptium, OS_ARCH=x86_64}
2025-12-03T12:40:06.382-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClient] Executing build b7e719cd-a1f4-4ffa-afa1-717912b2c4ba in daemon client {pid=9787}
2025-12-03T12:40:06.397-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface en5
2025-12-03T12:40:06.397-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.397-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:a0a8:4eff:fe33:354a%en5
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface utun3
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:ce81:b1c:bd2c:69e%utun3
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface utun2
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:4c97:77b:c002:6446%utun2
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface utun1
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:106b:e26:24eb:846d%utun1
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface utun0
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.398-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:acec:1cdc:528d:72c6%utun0
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface llw0
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:7079:48ff:feff:83cc%llw0
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface ap1
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:2023:35ff:fe51:2a67%ap1
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface en3
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:aede:48ff:fe00:1122%en3
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface en6
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:1857:8fe9:ac43:742d%en6
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /169.254.231.141
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface en4
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? false
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /2600:381:b508:5a:d44e:a988:b243:7d80%en4
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /2600:381:b508:5a:c4c:591f:6458:66af%en4
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /fe80:0:0:0:8c:d6af:73:21f3%en4
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding remote address /172.20.10.10
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding IP addresses for network interface lo0
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Is this a loopback interface? true
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Ignoring remote address on loopback interface /fe80:0:0:0:0:0:0:1%lo0
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding loopback address /0:0:0:0:0:0:0:1%lo0
2025-12-03T12:40:06.399-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.InetAddresses] Adding loopback address /127.0.0.1
2025-12-03T12:40:06.427-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Waiting to acquire shared lock on daemon addresses registry.
2025-12-03T12:40:06.430-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Lock acquired on daemon addresses registry.
2025-12-03T12:40:06.436-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Releasing lock on daemon addresses registry.
2025-12-03T12:40:06.439-0600 [INFO] [org.gradle.launcher.daemon.client.DefaultDaemonConnector] Found daemon DaemonInfo{pid=1493, address=[f6b72d31-b58a-4fe1-8f9d-2c84fba69bf3 port:49317, addresses:[/127.0.0.1]], state=Idle, lastBusy=1764787157477, context=DefaultDaemonContext[uid=1bb8a592-74c7-49fd-9667-9fcb83bcebf2,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1493,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=utf8,-Duser.country=US,-Duser.language=en,-Duser.variant]} however its context does not match the desired criteria.
At least one daemon option is different.
Wanted: DaemonRequestContext{jvmCriteria=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home (no JDK specified, using current Java home), daemonOpts=[--add-opens=java.base/java.util=ALL-UNNAMED, --add-opens=java.base/java.lang=ALL-UNNAMED, --add-opens=java.base/java.lang.invoke=ALL-UNNAMED, --add-opens=java.prefs/java.util.prefs=ALL-UNNAMED, --add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED, --add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED, --add-opens=java.base/java.nio.charset=ALL-UNNAMED, --add-opens=java.base/java.net=ALL-UNNAMED, --add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED, -XX:MaxMetaspaceSize=384m, -XX:+HeapDumpOnOutOfMemoryError, -Xms256m, -Xmx512m, -Dfile.encoding=UTF-8, -Duser.country=US, -Duser.language=en, -Duser.variant], applyInstrumentationAgent=true, nativeServicesMode=ENABLED, priority=NORMAL}
Actual: DefaultDaemonContext[uid=1bb8a592-74c7-49fd-9667-9fcb83bcebf2,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1493,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=utf8,-Duser.country=US,-Duser.language=en,-Duser.variant]

  Looking for a different daemon...
2025-12-03T12:40:06.441-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.TcpOutgoingConnector] Attempting to connect to [7109e125-342f-4ec3-b525-9f055a61bcb3 port:49307, addresses:[/127.0.0.1]].
2025-12-03T12:40:06.442-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.TcpOutgoingConnector] Trying to connect to address /127.0.0.1.
2025-12-03T12:40:06.448-0600 [DEBUG] [org.gradle.internal.remote.internal.inet.TcpOutgoingConnector] Connected to address /127.0.0.1:49307.
2025-12-03T12:40:06.483-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClient] Connected to daemon DaemonInfo{pid=1456, address=[7109e125-342f-4ec3-b525-9f055a61bcb3 port:49307, addresses:[/127.0.0.1]], state=Idle, lastBusy=1764787200080, context=DefaultDaemonContext[uid=6a7cdb50-ad0f-4d8a-bccb-46fc82be61f9,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1456,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=UTF-8,-Duser.country=US,-Duser.language=en,-Duser.variant]}. Dispatching request Build{id=b7e719cd-a1f4-4ffa-afa1-717912b2c4ba, currentDir=/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm}.
2025-12-03T12:40:06.483-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClientConnection] thread 1: dispatching class org.gradle.launcher.daemon.protocol.Build
2025-12-03T12:40:06.502-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClient] Received result org.gradle.launcher.daemon.protocol.BuildStarted@cf65451 from daemon DaemonInfo{pid=1456, address=[7109e125-342f-4ec3-b525-9f055a61bcb3 port:49307, addresses:[/127.0.0.1]], state=Idle, lastBusy=1764787200080, context=DefaultDaemonContext[uid=6a7cdb50-ad0f-4d8a-bccb-46fc82be61f9,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1456,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=UTF-8,-Duser.country=US,-Duser.language=en,-Duser.variant]} (build should be starting).
2025-12-03T12:40:06.501-0600 [INFO] [org.gradle.launcher.daemon.server.exec.LogToClient] The client will now receive all logging from the daemon (pid: 1456). The daemon log file: /Users/mymac/.gradle/daemon/8.10.2/daemon-1456.out.log
2025-12-03T12:40:06.501-0600 [INFO] [org.gradle.launcher.daemon.server.exec.LogAndCheckHealth] Starting 55th build in daemon [uptime: 1 hrs 26 mins 55.35 secs, performance: 99%, GC rate: 0.00/s, heap usage: 0% of 512 MiB, non-heap usage: 37% of 384 MiB]
2025-12-03T12:40:06.502-0600 [DEBUG] [org.gradle.launcher.daemon.server.exec.ExecuteBuild] The daemon has started executing the build.
2025-12-03T12:40:06.502-0600 [DEBUG] [org.gradle.launcher.daemon.server.exec.ExecuteBuild] Executing build with daemon context: DefaultDaemonContext[uid=6a7cdb50-ad0f-4d8a-bccb-46fc82be61f9,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1456,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=UTF-8,-Duser.country=US,-Duser.language=en,-Duser.variant]
2025-12-03T12:40:06.503-0600 [INFO] [org.gradle.internal.work.DefaultWorkerLeaseService] Using 8 worker leases.
2025-12-03T12:40:06.503-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: acquired lock on worker lease
2025-12-03T12:40:06.503-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Run build' started
2025-12-03T12:40:06.507-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for fileHashes, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/fileHashes/fileHashes.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@25d9698e
2025-12-03T12:40:06.508-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Acquiring file lock for file hash cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/fileHashes)
2025-12-03T12:40:06.508-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Waiting to acquire exclusive lock on file hash cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/fileHashes).
2025-12-03T12:40:06.508-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Lock acquired on file hash cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/fileHashes).
2025-12-03T12:40:06.509-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for outputFiles, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/buildOutputCleanup/outputFiles.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@5754846
2025-12-03T12:40:06.509-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Acquiring file lock for Build Output Cleanup Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/buildOutputCleanup)
2025-12-03T12:40:06.510-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Waiting to acquire exclusive lock on Build Output Cleanup Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/buildOutputCleanup).
2025-12-03T12:40:06.510-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Lock acquired on Build Output Cleanup Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/buildOutputCleanup).
2025-12-03T12:40:06.511-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for resourceHashesCache, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/fileHashes/resourceHashesCache.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@25d9698e
2025-12-03T12:40:06.512-0600 [INFO] [org.gradle.internal.watch.registry.impl.WatchableHierarchies] Now considering [/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm] as hierarchies to watch
2025-12-03T12:40:06.513-0600 [INFO] [org.gradle.tooling.internal.provider.FileSystemWatchingBuildActionRunner] Watching the file system is configured to be enabled if available
2025-12-03T12:40:06.513-0600 [DEBUG] [org.gradle.tooling.internal.provider.FileSystemWatchingBuildActionRunner] Watching the file system computed to be enabled if available
2025-12-03T12:40:06.513-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Build started for file system watching' started
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected apfs: / from /dev/disk1s4s1 (remote: false)
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected devfs: /dev from devfs (remote: false)
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected apfs: /System/Volumes/Preboot from /dev/disk1s2 (remote: false)
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected apfs: /System/Volumes/VM from /dev/disk1s6 (remote: false)
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected apfs: /System/Volumes/Update from /dev/disk1s5 (remote: false)
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected apfs: /System/Volumes/Data from /dev/disk1s1 (remote: false)
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected autofs: /System/Volumes/Data/home from map auto_home (remote: true)
2025-12-03T12:40:06.514-0600 [DEBUG] [org.gradle.internal.watch.vfs.impl.DefaultWatchableFileSystemDetector] Detected nullfs: /private/var/folders/nw/36byqj_s4_v5bz8zd_w52_680000gn/T/AppTranslocation/AF9938A7-B1B4-41F1-B623-E62B49E30620 from /Users/mymac/Downloads/Visual Studio Code.app (remote: false)
2025-12-03T12:40:06.515-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Build started for file system watching'
2025-12-03T12:40:06.515-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Build started for file system watching' completed
2025-12-03T12:40:06.515-0600 [INFO] [org.gradle.tooling.internal.provider.FileSystemWatchingBuildActionRunner] File system watching is active
2025-12-03T12:40:06.515-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Load build' started
2025-12-03T12:40:06.516-0600 [DEBUG] [org.gradle.initialization.properties.DefaultProjectPropertiesLoader] Found env project properties: []
2025-12-03T12:40:06.516-0600 [DEBUG] [org.gradle.initialization.properties.DefaultProjectPropertiesLoader] Found system project properties: []
2025-12-03T12:40:06.516-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Evaluate settings' started
2025-12-03T12:40:06.517-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for workingDirs, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/vcsMetadata/workingDirs.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@257d46ad
2025-12-03T12:40:06.524-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for md5-checksums, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums/md5-checksums.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@1eddf5ed
2025-12-03T12:40:06.525-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Acquiring file lock for checksums cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums)
2025-12-03T12:40:06.525-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Waiting to acquire exclusive lock on checksums cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums).
2025-12-03T12:40:06.525-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Lock acquired on checksums cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums).
2025-12-03T12:40:06.526-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for sha1-checksums, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums/sha1-checksums.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@1eddf5ed
2025-12-03T12:40:06.526-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for sha256-checksums, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums/sha256-checksums.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@1eddf5ed
2025-12-03T12:40:06.526-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for sha512-checksums, path /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums/sha512-checksums.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@1eddf5ed
2025-12-03T12:40:06.526-0600 [DEBUG] [org.gradle.api.internal.artifacts.mvnsettings.DefaultLocalMavenRepositoryLocator] No local repository in Settings file defined. Using default path: /Users/mymac/.m2/repository
2025-12-03T12:40:06.527-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for md-rule, path /Users/mymac/.gradle/caches/8.10.2/md-rule/md-rule.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@6c60d645
2025-12-03T12:40:06.527-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Acquiring file lock for cache directory md-rule (/Users/mymac/.gradle/caches/8.10.2/md-rule)
2025-12-03T12:40:06.527-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Waiting to acquire exclusive lock on cache directory md-rule (/Users/mymac/.gradle/caches/8.10.2/md-rule).
2025-12-03T12:40:06.528-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Lock acquired on cache directory md-rule (/Users/mymac/.gradle/caches/8.10.2/md-rule).
2025-12-03T12:40:06.528-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Creating new cache for md-supplier, path /Users/mymac/.gradle/caches/8.10.2/md-supplier/md-supplier.bin, access org.gradle.cache.internal.DefaultCacheCoordinator@57b59bda
2025-12-03T12:40:06.529-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Acquiring file lock for cache directory md-supplier (/Users/mymac/.gradle/caches/8.10.2/md-supplier)
2025-12-03T12:40:06.529-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Waiting to acquire exclusive lock on cache directory md-supplier (/Users/mymac/.gradle/caches/8.10.2/md-supplier).
2025-12-03T12:40:06.529-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Lock acquired on cache directory md-supplier (/Users/mymac/.gradle/caches/8.10.2/md-supplier).
2025-12-03T12:40:06.529-0600 [DEBUG] [org.gradle.internal.locking.LockFileReaderWriter] Lockfiles root: /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/gradle/dependency-locks
2025-12-03T12:40:06.530-0600 [INFO] [org.gradle.internal.buildevents.BuildLogger] Starting Build
2025-12-03T12:40:06.530-0600 [DEBUG] [org.gradle.internal.buildevents.BuildLogger] Gradle user home: /Users/mymac/.gradle
2025-12-03T12:40:06.530-0600 [DEBUG] [org.gradle.internal.buildevents.BuildLogger] Current dir: /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm
2025-12-03T12:40:06.531-0600 [DEBUG] [org.gradle.internal.buildevents.BuildLogger] Settings file: null
2025-12-03T12:40:06.532-0600 [DEBUG] [org.gradle.internal.buildevents.BuildLogger] Build file: null
2025-12-03T12:40:06.533-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply settings file 'settings.gradle.kts' to settings 'ron-app-sdk-jvm'' started
2025-12-03T12:40:06.533-0600 [DEBUG] [org.gradle.kotlin.dsl.provider.KotlinScriptPlugin] Applying Kotlin script to settings 'ron-app-sdk-jvm'
2025-12-03T12:40:06.534-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Apply settings file 'settings.gradle.kts' to settings 'ron-app-sdk-jvm''
2025-12-03T12:40:06.534-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply settings file 'settings.gradle.kts' to settings 'ron-app-sdk-jvm'' completed
2025-12-03T12:40:06.534-0600 [DEBUG] [org.gradle.initialization.ScriptEvaluatingSettingsProcessor] Timing: Processing settings took: 0.018 secs
2025-12-03T12:40:06.534-0600 [INFO] [org.gradle.internal.buildevents.BuildLogger] Settings evaluated using settings file '/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/settings.gradle.kts'.
2025-12-03T12:40:06.535-0600 [DEBUG] [org.gradle.caching.configuration.internal.DefaultBuildCacheConfiguration] Found class org.gradle.caching.local.internal.DirectoryBuildCacheServiceFactory registered for class org.gradle.caching.local.DirectoryBuildCache
2025-12-03T12:40:06.535-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Finalize build cache configuration' started
2025-12-03T12:40:06.535-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Finalize build cache configuration'
2025-12-03T12:40:06.535-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Finalize build cache configuration' completed
2025-12-03T12:40:06.535-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Evaluate settings'
2025-12-03T12:40:06.535-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Evaluate settings' completed
2025-12-03T12:40:06.536-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Load build'
2025-12-03T12:40:06.536-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Load build' completed
2025-12-03T12:40:06.536-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Configure build' started
2025-12-03T12:40:06.537-0600 [DEBUG] [org.gradle.internal.locking.LockFileReaderWriter] Lockfiles root: /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/gradle/dependency-locks
2025-12-03T12:40:06.538-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of incomingCatalogForLibs0' started
2025-12-03T12:40:06.540-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration unspecified:unspecified:unspecified(incomingCatalogForLibs0).
2025-12-03T12:40:06.541-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.oldresult.TransientConfigurationResultsBuilder] Flushing resolved configuration data in Binary store in /Users/mymac/.gradle/.tmp/gradle6826399413432578384.bin. Wrote root 3.
2025-12-03T12:40:06.541-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve dependencies of incomingCatalogForLibs0'
2025-12-03T12:40:06.541-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of incomingCatalogForLibs0' completed
2025-12-03T12:40:06.541-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'incomingCatalogForLibs0'' started
2025-12-03T12:40:06.542-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match libs.versions.toml from candidates [libs.versions.toml] for {org.gradle.category=platform, org.gradle.usage=version-catalog}
2025-12-03T12:40:06.542-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration 'incomingCatalogForLibs0''
2025-12-03T12:40:06.542-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'incomingCatalogForLibs0'' completed
2025-12-03T12:40:06.547-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.547-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.547-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.548-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Load projects' started
2025-12-03T12:40:06.549-0600 [DEBUG] [org.gradle.initialization.ProjectPropertySettingBuildLoader] Looking for project properties from: /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/gradle.properties
2025-12-03T12:40:06.549-0600 [DEBUG] [org.gradle.initialization.ProjectPropertySettingBuildLoader] project property file does not exists. We continue!
2025-12-03T12:40:06.550-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Load projects'
2025-12-03T12:40:06.550-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Load projects' completed
2025-12-03T12:40:06.550-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify projectsLoaded listeners' started
2025-12-03T12:40:06.550-0600 [INFO] [org.gradle.internal.buildevents.BuildLogger] Projects loaded. Root project using build file '/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/build.gradle.kts'.
2025-12-03T12:40:06.550-0600 [INFO] [org.gradle.internal.buildevents.BuildLogger] Included projects: [root project 'ron-app-sdk-jvm']
2025-12-03T12:40:06.550-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Notify projectsLoaded listeners'
2025-12-03T12:40:06.551-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify projectsLoaded listeners' completed
2025-12-03T12:40:06.551-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Configure project :' started
2025-12-03T12:40:06.551-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify beforeEvaluate listeners of :' started
2025-12-03T12:40:06.552-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Notify beforeEvaluate listeners of :'
2025-12-03T12:40:06.553-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.help-tasks to root project 'ron-app-sdk-jvm'' started
2025-12-03T12:40:06.555-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :help' started
2025-12-03T12:40:06.555-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :help'
2025-12-03T12:40:06.555-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :projects' started
2025-12-03T12:40:06.555-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :projects'
2025-12-03T12:40:06.556-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :tasks' started
2025-12-03T12:40:06.556-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :tasks'
2025-12-03T12:40:06.556-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :properties' started
2025-12-03T12:40:06.556-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :properties'
2025-12-03T12:40:06.557-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :dependencyInsight' started
2025-12-03T12:40:06.557-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :dependencyInsight'
2025-12-03T12:40:06.557-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :dependencies' started
2025-12-03T12:40:06.558-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :dependencies'
2025-12-03T12:40:06.559-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :buildEnvironment' started
2025-12-03T12:40:06.559-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :buildEnvironment'
2025-12-03T12:40:06.560-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :components' started
2025-12-03T12:40:06.560-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :components'
2025-12-03T12:40:06.569-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :model' started
2025-12-03T12:40:06.569-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :model'
2025-12-03T12:40:06.569-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :dependentComponents' started
2025-12-03T12:40:06.570-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :dependentComponents'
2025-12-03T12:40:06.570-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :outgoingVariants' started
2025-12-03T12:40:06.571-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :outgoingVariants'
2025-12-03T12:40:06.571-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :resolvableConfigurations' started
2025-12-03T12:40:06.571-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :resolvableConfigurations'
2025-12-03T12:40:06.572-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Apply plugin org.gradle.help-tasks to root project 'ron-app-sdk-jvm''
2025-12-03T12:40:06.572-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :javaToolchains' started
2025-12-03T12:40:06.572-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :javaToolchains'
2025-12-03T12:40:06.573-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :prepareKotlinBuildScriptModel' started
2025-12-03T12:40:06.574-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :prepareKotlinBuildScriptModel'
2025-12-03T12:40:06.575-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :updateDaemonJvm' started
2025-12-03T12:40:06.575-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :updateDaemonJvm'
2025-12-03T12:40:06.578-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.build-init to root project 'ron-app-sdk-jvm'' started
2025-12-03T12:40:06.578-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :init' started
2025-12-03T12:40:06.578-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :init'
2025-12-03T12:40:06.579-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Apply plugin org.gradle.build-init to root project 'ron-app-sdk-jvm''
2025-12-03T12:40:06.585-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.wrapper to root project 'ron-app-sdk-jvm'' started
2025-12-03T12:40:06.586-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :wrapper' started
2025-12-03T12:40:06.586-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :wrapper'
2025-12-03T12:40:06.586-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Apply plugin org.gradle.wrapper to root project 'ron-app-sdk-jvm''
2025-12-03T12:40:06.590-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply build file 'build.gradle.kts' to root project 'ron-app-sdk-jvm'' started
2025-12-03T12:40:06.597-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of detachedConfiguration1' started
2025-12-03T12:40:06.603-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve dependencies of detachedConfiguration1'
2025-12-03T12:40:06.605-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'detachedConfiguration1'' started
2025-12-03T12:40:06.605-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration 'detachedConfiguration1''
2025-12-03T12:40:06.607-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of detachedConfiguration2' started
2025-12-03T12:40:06.611-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve dependencies of detachedConfiguration2'
2025-12-03T12:40:06.614-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'detachedConfiguration2'' started
2025-12-03T12:40:06.615-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration 'detachedConfiguration2''
2025-12-03T12:40:06.616-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of detachedConfiguration3' started
2025-12-03T12:40:06.621-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve dependencies of detachedConfiguration3'
2025-12-03T12:40:06.622-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'detachedConfiguration3'' started
2025-12-03T12:40:06.622-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration 'detachedConfiguration3''
2025-12-03T12:40:06.625-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of :classpath' started
2025-12-03T12:40:06.732-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve dependencies of :classpath'
2025-12-03T12:40:06.739-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' started
2025-12-03T12:40:06.758-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 2: acquired lock on worker lease
2025-12-03T12:40:06.758-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25)' started
2025-12-03T12:40:06.758-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations: acquired lock on worker lease
2025-12-03T12:40:06.759-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25)' started
2025-12-03T12:40:06.759-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 3: acquired lock on worker lease
2025-12-03T12:40:06.759-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25)' started
2025-12-03T12:40:06.760-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25)'
2025-12-03T12:40:06.760-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25)' completed
2025-12-03T12:40:06.760-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25)'
2025-12-03T12:40:06.761-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1)' started
2025-12-03T12:40:06.761-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25)' completed
2025-12-03T12:40:06.761-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25)' started
2025-12-03T12:40:06.761-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25)'
2025-12-03T12:40:06.761-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 7: acquired lock on worker lease
2025-12-03T12:40:06.762-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25)' completed
2025-12-03T12:40:06.764-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25)' started
2025-12-03T12:40:06.762-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25)' started
2025-12-03T12:40:06.765-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1)'
2025-12-03T12:40:06.764-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25)'
2025-12-03T12:40:06.765-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25)' completed
2025-12-03T12:40:06.764-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25)' started
2025-12-03T12:40:06.764-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 6: acquired lock on worker lease
2025-12-03T12:40:06.766-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25)' started
2025-12-03T12:40:06.765-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25)'
2025-12-03T12:40:06.769-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25)' completed
2025-12-03T12:40:06.770-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25)'
2025-12-03T12:40:06.770-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25)' completed
2025-12-03T12:40:06.771-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25)' started
2025-12-03T12:40:06.771-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25)' started
2025-12-03T12:40:06.769-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25)' started
2025-12-03T12:40:06.768-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25)'
2025-12-03T12:40:06.772-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25)' completed
2025-12-03T12:40:06.768-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25)' started
2025-12-03T12:40:06.768-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 5: acquired lock on worker lease
2025-12-03T12:40:06.772-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25)' started
2025-12-03T12:40:06.771-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25)'
2025-12-03T12:40:06.774-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25)' completed
2025-12-03T12:40:06.774-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25)'
2025-12-03T12:40:06.773-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25)'
2025-12-03T12:40:06.774-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25)' completed
2025-12-03T12:40:06.773-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25)'
2025-12-03T12:40:06.775-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25)' completed
2025-12-03T12:40:06.773-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 4: acquired lock on worker lease
2025-12-03T12:40:06.773-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25)' started
2025-12-03T12:40:06.775-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6)' started
2025-12-03T12:40:06.775-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25)'
2025-12-03T12:40:06.776-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25)' completed
2025-12-03T12:40:06.775-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25)'
2025-12-03T12:40:06.776-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25)' completed
2025-12-03T12:40:06.775-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25)' started
2025-12-03T12:40:06.775-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25)' started
2025-12-03T12:40:06.774-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25)' started
2025-12-03T12:40:06.774-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25)' started
2025-12-03T12:40:06.777-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25)'
2025-12-03T12:40:06.778-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25)' completed
2025-12-03T12:40:06.778-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25)' started
2025-12-03T12:40:06.779-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25)'
2025-12-03T12:40:06.779-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25)' completed
2025-12-03T12:40:06.776-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330)' started
2025-12-03T12:40:06.776-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25)' started
2025-12-03T12:40:06.780-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25)'
2025-12-03T12:40:06.781-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25)' completed
2025-12-03T12:40:06.780-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6)'
2025-12-03T12:40:06.781-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6)' completed
2025-12-03T12:40:06.780-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330)'
2025-12-03T12:40:06.782-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330)' completed
2025-12-03T12:40:06.779-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0)' started
2025-12-03T12:40:06.784-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25)'
2025-12-03T12:40:06.784-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25)' completed
2025-12-03T12:40:06.784-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25)'
2025-12-03T12:40:06.783-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25)'
2025-12-03T12:40:06.785-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25)' completed
2025-12-03T12:40:06.782-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25)'
2025-12-03T12:40:06.782-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 7: released lock on worker lease
2025-12-03T12:40:06.781-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25)' started
2025-12-03T12:40:06.781-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25)' started
2025-12-03T12:40:06.785-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 2: released lock on worker lease
2025-12-03T12:40:06.785-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25)' completed
2025-12-03T12:40:06.785-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0)'
2025-12-03T12:40:06.786-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25)'
2025-12-03T12:40:06.787-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25)' completed
2025-12-03T12:40:06.787-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25)'
2025-12-03T12:40:06.787-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25)' completed
2025-12-03T12:40:06.786-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0)' completed
2025-12-03T12:40:06.787-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 3: released lock on worker lease
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations: released lock on worker lease
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 4: released lock on worker lease
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 5: released lock on worker lease
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 6: released lock on worker lease
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration ':classpath''
2025-12-03T12:40:06.800-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' started
2025-12-03T12:40:06.800-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Fingerprint transform inputs' started
2025-12-03T12:40:06.800-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Fingerprint transform inputs'
2025-12-03T12:40:06.809-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' started
2025-12-03T12:40:06.809-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.809-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.810-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.810-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.810-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.810-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.811-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.811-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.811-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.812-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.812-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.812-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.812-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.814-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.814-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.814-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.815-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.815-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.815-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.816-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.816-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.816-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.817-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.817-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.817-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.818-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.818-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.819-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.819-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.820-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.820-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.820-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.821-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.821-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.821-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.821-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.823-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.823-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.823-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.823-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.823-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.824-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.824-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration ':classpath''
2025-12-03T12:40:06.825-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Fingerprint transform inputs' started
2025-12-03T12:40:06.825-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Fingerprint transform inputs'
2025-12-03T12:40:06.826-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Fingerprint transform inputs' started
2025-12-03T12:40:06.826-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Fingerprint transform inputs'
2025-12-03T12:40:06.826-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.826-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.827-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: acquired lock on worker lease
2025-12-03T12:40:06.827-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.827-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.828-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.828-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.828-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.829-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: acquired lock on worker lease
2025-12-03T12:40:06.829-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.829-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.829-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.831-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.831-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.835-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: acquired lock on worker lease
2025-12-03T12:40:06.830-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.835-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.835-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.836-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.837-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.837-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.837-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.836-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.836-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.838-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.838-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.837-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.837-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.839-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.838-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.839-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.839-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.838-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.839-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.840-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.840-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.841-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.841-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.841-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.842-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.841-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 11: acquired lock on worker lease
2025-12-03T12:40:06.841-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.843-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.842-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.843-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.844-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 12: acquired lock on worker lease
2025-12-03T12:40:06.843-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.843-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.844-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.844-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.844-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.844-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.845-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.845-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.847-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.847-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.847-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.846-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.846-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.851-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.851-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.851-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.852-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.846-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.853-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.853-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.853-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.853-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.854-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: released lock on worker lease
2025-12-03T12:40:06.846-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.846-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.854-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.852-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.848-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.848-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.855-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.855-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.855-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.856-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.855-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.855-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: released lock on worker lease
2025-12-03T12:40:06.857-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.857-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.858-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.860-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.860-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.860-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.861-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.861-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.861-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.860-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.862-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.863-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.864-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.865-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.866-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.868-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.861-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.870-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.861-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: acquired lock on worker lease
2025-12-03T12:40:06.876-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.876-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.877-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.878-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.863-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.877-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.877-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 12: released lock on worker lease
2025-12-03T12:40:06.879-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.880-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.880-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.879-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.880-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.881-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.881-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.880-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.879-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: released lock on worker lease
2025-12-03T12:40:06.884-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: acquired lock on worker lease
2025-12-03T12:40:06.884-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.884-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.884-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.885-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.885-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.885-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.886-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.882-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.881-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.886-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.889-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.886-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: acquired lock on worker lease
2025-12-03T12:40:06.890-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.890-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.891-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.891-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.891-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.892-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.890-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 13: acquired lock on worker lease
2025-12-03T12:40:06.889-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.889-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.893-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.892-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.892-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.892-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.897-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.898-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.898-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.896-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.896-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.896-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.899-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.896-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.899-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.899-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.898-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 14: acquired lock on worker lease
2025-12-03T12:40:06.900-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 11: released lock on worker lease
2025-12-03T12:40:06.900-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.900-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.901-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.901-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.901-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.901-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.900-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.902-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.903-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.904-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.904-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.902-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.902-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.902-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.901-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.905-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.905-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.905-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.903-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.907-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.906-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.906-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.906-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.907-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.907-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.907-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.908-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.908-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.909-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.909-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.908-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 12: acquired lock on worker lease
2025-12-03T12:40:06.910-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.910-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.910-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.912-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.910-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.909-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.912-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.910-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.910-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: released lock on worker lease
2025-12-03T12:40:06.910-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.914-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: released lock on worker lease
2025-12-03T12:40:06.913-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.913-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.915-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.915-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.915-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 11: acquired lock on worker lease
2025-12-03T12:40:06.914-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.916-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.916-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.916-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.915-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.915-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.917-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.917-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: released lock on worker lease
2025-12-03T12:40:06.917-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.917-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.917-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.917-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.916-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.918-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.918-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.919-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.918-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.919-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.921-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.919-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: acquired lock on worker lease
2025-12-03T12:40:06.921-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.921-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 14: released lock on worker lease
2025-12-03T12:40:06.921-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.920-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.922-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.922-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.922-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.921-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.923-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.922-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.922-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.924-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.923-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: acquired lock on worker lease
2025-12-03T12:40:06.923-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.925-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.925-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.924-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.924-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.924-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.924-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.926-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.927-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.926-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.926-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.926-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.926-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.927-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.927-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.927-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.927-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 13: released lock on worker lease
2025-12-03T12:40:06.929-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.929-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.928-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.928-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.929-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.929-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.931-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.929-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: acquired lock on worker lease
2025-12-03T12:40:06.931-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.931-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.932-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.932-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.934-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.931-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.933-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.933-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.933-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 14: acquired lock on worker lease
2025-12-03T12:40:06.935-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.934-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 11: released lock on worker lease
2025-12-03T12:40:06.934-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.935-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.934-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.934-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.934-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.936-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.936-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.935-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.935-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.938-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.937-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.937-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.937-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.939-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.939-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.939-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.939-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 13: acquired lock on worker lease
2025-12-03T12:40:06.938-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.940-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.940-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.940-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.940-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.939-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.942-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.942-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.941-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.941-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.941-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.941-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.941-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.943-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.944-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.943-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.943-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.942-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: released lock on worker lease
2025-12-03T12:40:06.942-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.945-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 11: acquired lock on worker lease
2025-12-03T12:40:06.945-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.945-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.944-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.943-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.946-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.946-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.946-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.948-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.949-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.951-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.951-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.951-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.951-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.952-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.952-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.952-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 14: released lock on worker lease
2025-12-03T12:40:06.946-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.953-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.945-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.953-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.948-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.954-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.947-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.947-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.947-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.955-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.954-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: released lock on worker lease
2025-12-03T12:40:06.954-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.954-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: acquired lock on worker lease
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.955-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.959-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.959-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.959-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.959-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.960-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.960-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.960-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.960-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.960-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: released lock on worker lease
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.961-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.962-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.962-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.963-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.963-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.963-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.964-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.964-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.966-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.967-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.967-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.967-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.968-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.968-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.968-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.971-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.973-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: released lock on worker lease
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.974-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.974-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.975-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.976-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.977-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.977-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.963-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.979-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.979-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.980-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.978-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.980-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.978-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 12: released lock on worker lease
2025-12-03T12:40:06.983-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.980-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.989-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.989-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.989-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:06.990-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:06.990-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.993-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.993-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.994-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.995-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.996-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.996-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.997-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:06.997-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.006-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.009-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:06.982-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 13: released lock on worker lease
2025-12-03T12:40:07.019-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 13: acquired lock on worker lease
2025-12-03T12:40:07.026-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:07.026-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.026-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.027-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.026-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 14: acquired lock on worker lease
2025-12-03T12:40:07.028-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:07.028-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.028-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.029-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 12: acquired lock on worker lease
2025-12-03T12:40:07.037-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:07.038-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.038-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.025-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:07.039-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.038-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: acquired lock on worker lease
2025-12-03T12:40:07.029-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.028-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.040-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.040-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: acquired lock on worker lease
2025-12-03T12:40:07.040-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:07.039-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.042-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.042-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.039-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.042-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.043-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: acquired lock on worker lease
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.044-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.040-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.044-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.043-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.043-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 11: released lock on worker lease
2025-12-03T12:40:07.043-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' started
2025-12-03T12:40:07.046-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.046-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.043-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.043-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.043-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.048-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.048-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.048-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.049-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.047-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.050-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.050-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.051-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.052-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.047-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.053-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.046-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.055-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.045-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.045-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.055-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.054-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.052-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.058-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.059-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:07.061-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:07.062-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 12: released lock on worker lease
2025-12-03T12:40:07.063-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.064-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.069-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.069-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.073-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.073-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.073-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:07.073-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:07.074-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 13: released lock on worker lease
2025-12-03T12:40:07.059-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.078-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.057-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.056-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.081-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.082-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.085-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.087-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.087-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.088-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.056-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.056-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.088-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.089-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.089-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.090-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.090-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.090-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:07.091-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:07.091-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 14: released lock on worker lease
2025-12-03T12:40:07.088-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.092-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.092-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.093-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:07.093-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:07.096-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 10: released lock on worker lease
2025-12-03T12:40:07.080-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.079-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.097-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.089-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.098-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.099-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.099-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.100-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.100-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.101-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.101-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.101-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.100-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.102-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.105-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.105-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.100-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.102-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.106-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:07.106-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:07.106-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.107-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 9: released lock on worker lease
2025-12-03T12:40:07.106-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.107-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.107-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:07.326-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.327-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.327-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute transform'
2025-12-03T12:40:07.327-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:07.327-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Unconstrained build operations Thread 8: released lock on worker lease
2025-12-03T12:40:07.329-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration ':classpath''
2025-12-03T12:40:07.329-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' started
2025-12-03T12:40:07.329-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Resolve files of configuration ':classpath''
2025-12-03T12:40:07.330-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.kotlin.dsl.provider.plugins.KotlinScriptBasePlugin to root project 'ron-app-sdk-jvm'' started
2025-12-03T12:40:07.331-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.kotlin.dsl.provider.plugins.KotlinScriptRootPlugin to root project 'ron-app-sdk-jvm'' started
2025-12-03T12:40:07.331-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Apply plugin org.gradle.kotlin.dsl.provider.plugins.KotlinScriptRootPlugin to root project 'ron-app-sdk-jvm''
2025-12-03T12:40:07.332-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :kotlinDslAccessorsReport' started
2025-12-03T12:40:07.332-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Register task :kotlinDslAccessorsReport'
2025-12-03T12:40:07.332-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Apply plugin org.gradle.kotlin.dsl.provider.plugins.KotlinScriptBasePlugin to root project 'ron-app-sdk-jvm''
2025-12-03T12:40:07.385-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' started
2025-12-03T12:40:07.387-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Identifying work'
2025-12-03T12:40:07.388-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute 'allprojects {}' action' started
2025-12-03T12:40:07.388-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Cross-configure project :' started
2025-12-03T12:40:07.389-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Cross-configure project :'
2025-12-03T12:40:07.389-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute 'allprojects {}' action'
2025-12-03T12:40:07.389-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute 'subprojects {}' action' started
2025-12-03T12:40:07.389-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Execute 'subprojects {}' action'
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Apply build file 'build.gradle.kts' to root project 'ron-app-sdk-jvm''
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify afterEvaluate listeners of :' started
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Notify afterEvaluate listeners of :'
2025-12-03T12:40:06.551-0600 [LIFECYCLE] [org.gradle.internal.logging.progress.ProgressLoggerFactory] 
2025-12-03T12:40:06.551-0600 [LIFECYCLE] [org.gradle.internal.logging.progress.ProgressLoggerFactory] > Configure project :
2025-12-03T12:40:06.551-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: acquired lock on state of build :
2025-12-03T12:40:06.552-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify beforeEvaluate listeners of :' completed
2025-12-03T12:40:06.554-0600 [DEBUG] [org.gradle.model.internal.registry.DefaultModelRegistry] Project : - Transitioning model element '<root>' from state Registered to Created
2025-12-03T12:40:06.554-0600 [DEBUG] [org.gradle.model.internal.registry.DefaultModelRegistry] Project : - Transitioning model element '<root>' to state Discovered.
2025-12-03T12:40:06.554-0600 [DEBUG] [org.gradle.model.internal.registry.DefaultModelRegistry] Project : - Transitioning model element '<root>' to state Created.
2025-12-03T12:40:06.555-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :help' completed
2025-12-03T12:40:06.555-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :projects' completed
2025-12-03T12:40:06.556-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :tasks' completed
2025-12-03T12:40:06.557-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :properties' completed
2025-12-03T12:40:06.557-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :dependencyInsight' completed
2025-12-03T12:40:06.559-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :dependencies' completed
2025-12-03T12:40:06.559-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :buildEnvironment' completed
2025-12-03T12:40:06.560-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :components' completed
2025-12-03T12:40:06.569-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :model' completed
2025-12-03T12:40:06.570-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :dependentComponents' completed
2025-12-03T12:40:06.571-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :outgoingVariants' completed
2025-12-03T12:40:06.571-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :resolvableConfigurations' completed
2025-12-03T12:40:06.572-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.help-tasks to root project 'ron-app-sdk-jvm'' completed
2025-12-03T12:40:06.572-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :javaToolchains' completed
2025-12-03T12:40:06.574-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :prepareKotlinBuildScriptModel' completed
2025-12-03T12:40:06.576-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :updateDaemonJvm' completed
2025-12-03T12:40:06.579-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :init' completed
2025-12-03T12:40:06.581-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.build-init to root project 'ron-app-sdk-jvm'' completed
2025-12-03T12:40:06.586-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :wrapper' completed
2025-12-03T12:40:06.586-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.wrapper to root project 'ron-app-sdk-jvm'' completed
2025-12-03T12:40:06.586-0600 [INFO] [org.gradle.configuration.project.BuildScriptProcessor] Evaluating root project 'ron-app-sdk-jvm' using build file '/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/build.gradle.kts'.
2025-12-03T12:40:06.589-0600 [DEBUG] [org.gradle.internal.locking.LockFileReaderWriter] Lockfiles root: /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/gradle/dependency-locks
2025-12-03T12:40:06.591-0600 [DEBUG] [org.gradle.kotlin.dsl.provider.KotlinScriptPlugin] Applying Kotlin script to root project 'ron-app-sdk-jvm'
2025-12-03T12:40:06.598-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.modulecache.ResolvedArtifactCaches] Creating new in-memory cache for repo 'Gradle Central Plugin Repository' [2f8748e965ecb2135dad092e9ef808de].
2025-12-03T12:40:06.600-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration unspecified:unspecified:unspecified(detachedConfiguration1).
2025-12-03T12:40:06.602-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.602-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.602-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.603-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [1] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4e6e89d8, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3fbe75a, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3e097c5e, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@34a6475b, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@e3c285e, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@52a1cff4, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@273fb272, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@21a5a551] for {}
2025-12-03T12:40:06.603-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25(runtime).
2025-12-03T12:40:06.603-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25(runtime) has no transitive incoming edges. ignoring outgoing edges.
2025-12-03T12:40:06.603-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.oldresult.TransientConfigurationResultsBuilder] Flushing resolved configuration data in Binary store in /Users/mymac/.gradle/.tmp/gradle6826399413432578384.bin. Wrote root 7.
2025-12-03T12:40:06.604-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of detachedConfiguration1' completed
2025-12-03T12:40:06.604-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 configuration runtime] for {}
2025-12-03T12:40:06.604-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Detected non-existence of artifact 'org.jetbrains.kotlin.jvm.gradle.plugin-1.9.25.jar (org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25)' in resolver cache
2025-12-03T12:40:06.605-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'detachedConfiguration1'' completed
2025-12-03T12:40:06.608-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.modulecache.ResolvedArtifactCaches] Reusing in-memory cache for repo 'Gradle Central Plugin Repository' [2f8748e965ecb2135dad092e9ef808de].
2025-12-03T12:40:06.609-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration unspecified:unspecified:unspecified(detachedConfiguration2).
2025-12-03T12:40:06.610-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.610-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.610-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.610-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1(runtime).
2025-12-03T12:40:06.610-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1(runtime) has no transitive incoming edges. ignoring outgoing edges.
2025-12-03T12:40:06.611-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.oldresult.TransientConfigurationResultsBuilder] Flushing resolved configuration data in Binary store in /Users/mymac/.gradle/.tmp/gradle6826399413432578384.bin. Wrote root 23.
2025-12-03T12:40:06.611-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of detachedConfiguration2' completed
2025-12-03T12:40:06.612-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 configuration runtime from candidates [org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 configuration runtime] for {}
2025-12-03T12:40:06.612-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Detected non-existence of artifact 'org.jlleitschuh.gradle.ktlint.gradle.plugin-12.1.1.jar (org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1)' in resolver cache
2025-12-03T12:40:06.615-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'detachedConfiguration2'' completed
2025-12-03T12:40:06.617-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.modulecache.ResolvedArtifactCaches] Reusing in-memory cache for repo 'Gradle Central Plugin Repository' [2f8748e965ecb2135dad092e9ef808de].
2025-12-03T12:40:06.618-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration unspecified:unspecified:unspecified(detachedConfiguration3).
2025-12-03T12:40:06.620-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.620-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.620-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.620-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6(runtime).
2025-12-03T12:40:06.620-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6(runtime) has no transitive incoming edges. ignoring outgoing edges.
2025-12-03T12:40:06.621-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.oldresult.TransientConfigurationResultsBuilder] Flushing resolved configuration data in Binary store in /Users/mymac/.gradle/.tmp/gradle6826399413432578384.bin. Wrote root 39.
2025-12-03T12:40:06.621-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of detachedConfiguration3' completed
2025-12-03T12:40:06.621-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 configuration runtime from candidates [io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 configuration runtime] for {}
2025-12-03T12:40:06.621-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Detected non-existence of artifact 'io.gitlab.arturbosch.detekt.gradle.plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6)' in resolver cache
2025-12-03T12:40:06.622-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration 'detachedConfiguration3'' completed
2025-12-03T12:40:06.623-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration :ron-app-sdk-jvm:unspecified(classpath).
2025-12-03T12:40:06.623-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] dependency: org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 to-conf: null is filtered.
2025-12-03T12:40:06.623-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] dependency: org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 to-conf: null is filtered.
2025-12-03T12:40:06.624-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] dependency: io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 to-conf: null is filtered.
2025-12-03T12:40:06.624-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] dependency: org.apache.logging.log4j:log4j-core:{require 2.17.1; reject [2.0, 2.17.1)} to-conf: null is filtered.
2025-12-03T12:40:06.624-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] dependency: org.jetbrains.kotlin:kotlin-stdlib:{strictly 1.9.24} to-conf: null is filtered.
2025-12-03T12:40:06.624-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] dependency: org.jetbrains.kotlin:kotlin-reflect:{strictly 1.9.24} to-conf: null is filtered.
2025-12-03T12:40:06.624-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] dependency: org.jetbrains:annotations:{strictly 13.0} to-conf: null is filtered.
2025-12-03T12:40:06.626-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.modulecache.ResolvedArtifactCaches] Reusing in-memory cache for repo 'Gradle Central Plugin Repository' [2f8748e965ecb2135dad092e9ef808de].
2025-12-03T12:40:06.626-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration :ron-app-sdk-jvm:unspecified(classpath).
2025-12-03T12:40:06.626-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.627-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.627-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.627-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4e6e89d8 attribute org.gradle.usage value {org.gradle.status=release, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.627-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3fbe75a doesn't have attribute org.gradle.dependency.bundling
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3fbe75a doesn't have attribute org.gradle.jvm.version
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3fbe75a doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3e097c5e attribute org.gradle.category value {org.gradle.category=documentation, org.gradle.status=release} doesn't requested value library
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@34a6475b attribute org.gradle.category value {org.gradle.category=documentation, org.gradle.status=release} doesn't requested value library
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@e3c285e attribute org.gradle.usage value {org.gradle.status=release, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@52a1cff4 attribute org.gradle.category value {org.gradle.category=platform, org.gradle.status=release, org.gradle.usage=java-runtime} doesn't requested value library
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@273fb272 attribute org.gradle.usage value {org.gradle.status=release, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.628-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@21a5a551 attribute org.gradle.category value {org.gradle.category=enforced-platform, org.gradle.status=release, org.gradle.usage=java-runtime} doesn't requested value library
2025-12-03T12:40:06.629-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [1] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4e6e89d8, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3fbe75a, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3e097c5e, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@34a6475b, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@e3c285e, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@52a1cff4, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@273fb272, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@21a5a551] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.629-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.629-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.629-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.630-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.630-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.630-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.630-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25(runtime).
2025-12-03T12:40:06.638-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.638-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.638-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.639-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@487c7567 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.639-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5942b6f8 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.639-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@7d632bcd doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.639-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5e21a337 attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.639-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@75dd32d9 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.639-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3964bfec attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.640-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5d3dc64e attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=7.0, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.640-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@e56eec5 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.640-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@12815e2c attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.640-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@e06084f attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=7.1, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.641-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3dbaa077 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.641-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@42341c3 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.641-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@1ccb1f55 attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=7.4, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.641-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@223128f3 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.642-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4a609dda attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.642-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@54d3e00c attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=7.5, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.642-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@10a67c74 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.642-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3bfba823 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.642-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@753b7ccb attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=7.6, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.643-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3ea331e0 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.643-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@1d45bf9c attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.643-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@7b99b6a7 attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.0, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.643-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@556a403b attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.643-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@72e15747 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.643-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4268afd8 attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.1, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.643-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@74f215bc attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.644-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@739ee881 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.644-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@b3dd2da attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.2, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.644-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [35] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@487c7567, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5942b6f8, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@7d632bcd, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5e21a337, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@75dd32d9, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3964bfec, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5d3dc64e, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@cacba0a, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@e56eec5, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@12815e2c, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@e06084f, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@681711d6, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3dbaa077, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@42341c3, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@1ccb1f55, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@6c4936b5, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@223128f3, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4a609dda, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@54d3e00c, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@98792e4, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@10a67c74, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3bfba823, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@753b7ccb, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@60920534, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3ea331e0, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@1d45bf9c, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@7b99b6a7, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@305b3151, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@556a403b, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@72e15747, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4268afd8, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@15825571, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@74f215bc, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@739ee881, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@b3dd2da, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@20a7f597] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.645-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1(runtime).
2025-12-03T12:40:06.646-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jlleitschuh.gradle:ktlint-gradle:12.1.1 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.646-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jlleitschuh.gradle:ktlint-gradle:12.1.1' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.647-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jlleitschuh.gradle:ktlint-gradle:12.1.1 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.647-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@23b90d attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.647-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@672d9caa doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.648-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@599e9323 doesn't have attribute org.gradle.jvm.version
2025-12-03T12:40:06.648-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@599e9323 doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.648-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@672d9caa selected because its attributes are a superset of all other candidate attributes
2025-12-03T12:40:06.649-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [1] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@23b90d, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@672d9caa, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@599e9323] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.649-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6(runtime).
2025-12-03T12:40:06.651-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.652-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.652-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.652-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@43821b44 attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.653-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5f93aca0 doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.653-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@759c021 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.653-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3bdc4838 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.653-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [1] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@43821b44, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@5f93aca0, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@759c021, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3bdc4838] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.653-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25(gradle82RuntimeElements).
2025-12-03T12:40:06.655-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.655-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.655-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.656-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@62d1a1cf attribute org.gradle.usage value {org.gradle.category=platform, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.656-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@41be987c doesn't have attribute org.gradle.libraryelements
2025-12-03T12:40:06.656-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@41be987c doesn't have attribute org.gradle.dependency.bundling
2025-12-03T12:40:06.656-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@41be987c doesn't have attribute org.gradle.jvm.version
2025-12-03T12:40:06.656-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@41be987c doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.656-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4b5f1553 attribute org.gradle.category value {org.gradle.category=enforced-platform} doesn't requested value platform
2025-12-03T12:40:06.657-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@74ce9e97 attribute org.gradle.category value {org.gradle.category=enforced-platform} doesn't requested value platform
2025-12-03T12:40:06.657-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [1] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@62d1a1cf, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@41be987c, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@4b5f1553, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@74ce9e97] for {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.676-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.677-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.677-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.677-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3fee6780 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.678-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@7da405c1 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.678-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@59dae2a8 doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.678-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@47193462 attribute org.gradle.usage value {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.environment=standard-jvm, org.gradle.jvm.version=8, org.gradle.libraryelements=jar, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.679-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [2] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@3fee6780, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@7da405c1, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@59dae2a8, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@47193462] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.679-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.679-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.679-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.680-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.680-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.680-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.680-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@6a808059 attribute org.gradle.usage value {org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.680-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@697653be doesn't have attribute org.gradle.category
2025-12-03T12:40:06.680-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@697653be doesn't have attribute org.gradle.libraryelements
2025-12-03T12:40:06.680-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@697653be doesn't have attribute org.gradle.dependency.bundling
2025-12-03T12:40:06.681-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@697653be doesn't have attribute org.gradle.jvm.version
2025-12-03T12:40:06.681-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@697653be doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.681-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@31ed9285 doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.681-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@6c430a91 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.681-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@2afcd08 attribute org.gradle.category value {org.gradle.category=documentation} doesn't requested value library
2025-12-03T12:40:06.682-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@31ed9285 selected because its attributes are a superset of all other candidate attributes
2025-12-03T12:40:06.682-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [2] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@6a808059, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@697653be, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@31ed9285, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@6c430a91, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@2afcd08] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.682-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.682-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.682-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-util-klib:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-util-klib:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-util-klib:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.683-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-android-extensions:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-android-extensions:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-android-extensions:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.684-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.685-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.685-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.685-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.685-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.685-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.686-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.686-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.687-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jlleitschuh.gradle:ktlint-gradle:12.1.1(runtimeElements).
2025-12-03T12:40:06.687-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6(runtimeElements).
2025-12-03T12:40:06.688-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25(runtimeElements).
2025-12-03T12:40:06.694-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.excludes.factories.LoggingExcludeFactory] {"operation": { "name": "anyOf", "operands": [{"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib-common"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-script-runtime"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-reflect"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib-jdk7"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib-jdk8"}], "result": { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]} } }
2025-12-03T12:40:06.695-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25(runtimeElementsWithFixedAttribute).
2025-12-03T12:40:06.696-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-project-model:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.696-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-project-model:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.696-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-project-model:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.696-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25(runtimeElements).
2025-12-03T12:40:06.700-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.700-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.700-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.700-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-native-utils:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.701-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-native-utils:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.701-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-native-utils:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.701-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-tooling-core:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.701-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-tooling-core:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.701-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-tooling-core:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.701-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25(runtimeElementsWithFixedAttribute).
2025-12-03T12:40:06.702-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25(runtimeElements).
2025-12-03T12:40:06.702-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25(runtime).
2025-12-03T12:40:06.702-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-util-klib:1.9.25(runtime).
2025-12-03T12:40:06.702-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-util-klib:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.705-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-util-io:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.705-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-util-io:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.706-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-util-io:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.707-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25(runtime).
2025-12-03T12:40:06.707-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.708-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25(runtime).
2025-12-03T12:40:06.708-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25(runtime).
2025-12-03T12:40:06.708-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.709-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-script-runtime is excluded from org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.709-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-reflect is excluded from org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.710-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.710-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.710-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.711-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.intellij.deps:trove4j:1.0.20200330 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.711-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.intellij.deps:trove4j:1.0.20200330' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.711-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.intellij.deps:trove4j:1.0.20200330 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.711-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-android-extensions:1.9.25(runtime).
2025-12-03T12:40:06.711-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25(runtime).
2025-12-03T12:40:06.712-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.excludes.factories.LoggingExcludeFactory] {"operation": { "name": "anyOf", "operands": [{"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib-common"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-script-runtime"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-reflect"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib-jdk7"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-build-common"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib"}, {"exclude module id" : "org.jetbrains.kotlin:kotlin-stdlib-jdk8"}], "result": { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-build-common", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]} } }
2025-12-03T12:40:06.712-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-build-common is excluded from org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-build-common", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.713-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-daemon-client:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.713-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-daemon-client:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.714-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-daemon-client:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.714-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.714-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.714-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.714-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@26738f3e attribute org.gradle.usage value {org.gradle.libraryelements=jar, org.gradle.usage=java-api} doesn't requested value java-runtime
2025-12-03T12:40:06.714-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@60520c03 doesn't have attribute org.gradle.category
2025-12-03T12:40:06.715-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@60520c03 doesn't have attribute org.gradle.dependency.bundling
2025-12-03T12:40:06.715-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@60520c03 doesn't have attribute org.gradle.jvm.version
2025-12-03T12:40:06.715-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@60520c03 doesn't have attribute org.gradle.plugin.api-version
2025-12-03T12:40:06.716-0600 [DEBUG] [org.gradle.internal.component.model.DefaultAttributeMatcher] Selected matches [1] from candidates [org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@26738f3e, org.gradle.internal.component.model.DefaultExternalComponentGraphResolveState$DefaultConfigurationGraphResolveState@60520c03] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.716-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25(runtime).
2025-12-03T12:40:06.717-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.717-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25(runtime).
2025-12-03T12:40:06.718-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.722-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-scripting-common:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.722-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-scripting-common:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.722-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-scripting-common:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.722-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Attempting to resolve component for org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25 using repositories [Gradle Central Plugin Repository]
2025-12-03T12:40:06.723-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Using cached module metadata for module 'org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25' in 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.723-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.RepositoryChainComponentMetaDataResolver] Using org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25 from Maven repository 'Gradle Central Plugin Repository'
2025-12-03T12:40:06.723-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25(gradle82RuntimeElements).
2025-12-03T12:40:06.723-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] Changed edges for org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25(gradle82RuntimeElements) selects same versions as previous traversal. ignoring
2025-12-03T12:40:06.723-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-project-model:1.9.25(runtime).
2025-12-03T12:40:06.723-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-project-model:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.724-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25(runtime).
2025-12-03T12:40:06.724-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-native-utils:1.9.25(runtime).
2025-12-03T12:40:06.724-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-tooling-core:1.9.25(runtime).
2025-12-03T12:40:06.724-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25(runtimeElements).
2025-12-03T12:40:06.724-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] Changed edges for org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25(runtimeElements) selects same versions as previous traversal. ignoring
2025-12-03T12:40:06.724-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25(runtimeElementsWithFixedAttribute).
2025-12-03T12:40:06.724-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] Changed edges for org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25(runtimeElementsWithFixedAttribute) selects same versions as previous traversal. ignoring
2025-12-03T12:40:06.725-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25(runtimeElements).
2025-12-03T12:40:06.725-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] Changed edges for org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25(runtimeElements) selects same versions as previous traversal. ignoring
2025-12-03T12:40:06.725-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-util-io:1.9.25(runtime).
2025-12-03T12:40:06.725-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-util-io:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.725-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25(runtime).
2025-12-03T12:40:06.725-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.intellij.deps:trove4j:1.0.20200330(runtime).
2025-12-03T12:40:06.725-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25(runtime).
2025-12-03T12:40:06.726-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.excludes.factories.LoggingExcludeFactory] {"operation": { "name": "allOf", "operands": [{ "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}, { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-build-common", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}], "result": { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]} } }
2025-12-03T12:40:06.727-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] Changed edges for org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25(runtime) selects same versions as previous traversal. ignoring
2025-12-03T12:40:06.727-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-daemon-client:1.9.25(runtime).
2025-12-03T12:40:06.727-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0(jvmRuntimeElements-published).
2025-12-03T12:40:06.727-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib-jdk8 is excluded from org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0(jvmRuntimeElements-published) by "excludes everything".
2025-12-03T12:40:06.727-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib-common is excluded from org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0(jvmRuntimeElements-published) by "excludes everything".
2025-12-03T12:40:06.727-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-scripting-common:1.9.25(runtime).
2025-12-03T12:40:06.728-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-scripting-common:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.728-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25(runtime).
2025-12-03T12:40:06.728-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-script-runtime is excluded from org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.728-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] org.jetbrains.kotlin:kotlin-stdlib is excluded from org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25(runtime) by { "module ids" : ["org.jetbrains.kotlin:kotlin-stdlib-common", "org.jetbrains.kotlin:kotlin-script-runtime", "org.jetbrains.kotlin:kotlin-reflect", "org.jetbrains.kotlin:kotlin-stdlib-jdk7", "org.jetbrains.kotlin:kotlin-stdlib", "org.jetbrains.kotlin:kotlin-stdlib-jdk8"]}.
2025-12-03T12:40:06.729-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-util-klib:1.9.25(runtime).
2025-12-03T12:40:06.729-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] Changed edges for org.jetbrains.kotlin:kotlin-util-klib:1.9.25(runtime) selects same versions as previous traversal. ignoring
2025-12-03T12:40:06.729-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.DependencyGraphBuilder] Visiting configuration org.jetbrains.kotlin:kotlin-scripting-common:1.9.25(runtime).
2025-12-03T12:40:06.729-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.graph.builder.NodeState] Changed edges for org.jetbrains.kotlin:kotlin-scripting-common:1.9.25(runtime) selects same versions as previous traversal. ignoring
2025-12-03T12:40:06.731-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.oldresult.TransientConfigurationResultsBuilder] Flushing resolved configuration data in Binary store in /Users/mymac/.gradle/.tmp/gradle6826399413432578384.bin. Wrote root 57.
2025-12-03T12:40:06.732-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve dependencies of :classpath' completed
2025-12-03T12:40:06.732-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.732-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 configuration runtime from candidates [org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.733-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 configuration runtime from candidates [io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.733-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25 variant runtimeElementsWithFixedAttribute from candidates [org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25 variant runtimeElementsWithFixedAttribute] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.733-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25 variant runtimeElementsWithFixedAttribute from candidates [org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25 variant runtimeElementsWithFixedAttribute] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.734-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements from candidates [org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements] for {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.734-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements from candidates [org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements] for {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.734-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements from candidates [org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements] for {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.735-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25 variant gradle82RuntimeElements from candidates [org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25 variant gradle82RuntimeElements] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.735-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jlleitschuh.gradle:ktlint-gradle:12.1.1 variant runtimeElements from candidates [org.jlleitschuh.gradle:ktlint-gradle:12.1.1 variant runtimeElements] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.735-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6 variant runtimeElements from candidates [io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6 variant runtimeElements] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.735-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.735-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25 variant runtimeElements from candidates [org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25 variant runtimeElements] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.736-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.736-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-native-utils:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-native-utils:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.736-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-util-klib:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-util-klib:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.737-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.737-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-android-extensions:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-android-extensions:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.737-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.737-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.737-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.738-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.738-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.738-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-project-model:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-project-model:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.738-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-tooling-core:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-tooling-core:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.738-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-util-io:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-util-io:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.738-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.738-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.intellij.deps:trove4j:1.0.20200330 configuration runtime from candidates [org.jetbrains.intellij.deps:trove4j:1.0.20200330 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.739-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-daemon-client:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-daemon-client:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.739-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0 variant jvmRuntimeElements-published from candidates [org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0 variant jvmRuntimeElements-published] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.739-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.739-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Selected match org.jetbrains.kotlin:kotlin-scripting-common:1.9.25 configuration runtime from candidates [org.jetbrains.kotlin:kotlin-scripting-common:1.9.25 configuration runtime] for {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.759-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'org.gradle.internal.component.external.model.UrlBackedArtifactMetadata@19b1dfdb' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-gradle-plugin-model/1.9.25/5293ae473d486683dea64bfbf945a9c191aa10ab/kotlin-gradle-plugin-model-1.9.25.jar
2025-12-03T12:40:06.760-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'org.gradle.internal.component.external.model.UrlBackedArtifactMetadata@51dbd465' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-gradle-plugin-api/1.9.25/8879c65d6c6d25e184cbdf8e64445e51bce90b15/kotlin-gradle-plugin-api-1.9.25.jar
2025-12-03T12:40:06.760-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'org.gradle.internal.component.external.model.UrlBackedArtifactMetadata@263b4654' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-gradle-plugin/1.9.25/71ab5536faa201ad35eb151332168b6a1b592f26/kotlin-gradle-plugin-1.9.25-gradle82.jar
2025-12-03T12:40:06.762-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-klib-commonizer-api/1.9.25/8adab8075ce5a08645292c68b80a6f5fc6d3881f/kotlin-klib-commonizer-api-1.9.25.jar
2025-12-03T12:40:06.762-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'org.gradle.internal.component.external.model.UrlBackedArtifactMetadata@5fc10392' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jlleitschuh.gradle/ktlint-gradle/12.1.1/94f8b340955b14387144c2d33f1e92cd3eebfa83/ktlint-gradle-12.1.1.jar
2025-12-03T12:40:06.765-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-build-tools-api/1.9.25/fedde3e3fd017081fff9678efeb5c3e7685d20df/kotlin-build-tools-api-1.9.25.jar
2025-12-03T12:40:06.766-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-gradle-plugin-idea-proto/1.9.25/9b30929018002a4a54543b868d94ba1059cd78b7/kotlin-gradle-plugin-idea-proto-1.9.25.jar
2025-12-03T12:40:06.769-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-android-extensions/1.9.25/66ea26e5bf05fbf1e78c2a2d3c4759058123435b/kotlin-android-extensions-1.9.25.jar
2025-12-03T12:40:06.765-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1)' completed
2025-12-03T12:40:06.765-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-util-klib/1.9.25/a77af11138d45106e1989a5134f22d96d3fd40eb/kotlin-util-klib-1.9.25.jar
2025-12-03T12:40:06.772-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-compiler-runner/1.9.25/15a37dca39870b75831f63aae82bb7ae2d7d1d02/kotlin-compiler-runner-1.9.25.jar
2025-12-03T12:40:06.772-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-compiler-embeddable/1.9.25/6e698f23ca1f74d6672827f61e92c2fe015d1b88/kotlin-compiler-embeddable-1.9.25.jar
2025-12-03T12:40:06.772-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-scripting-compiler-embeddable/1.9.25/bbc471c7ebad2f9179537c7044e917c276b49b0d/kotlin-scripting-compiler-embeddable-1.9.25.jar
2025-12-03T12:40:06.774-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25)' completed
2025-12-03T12:40:06.773-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-scripting-compiler-impl-embeddable/1.9.25/cdf3facb2388abb12962d2a501a879ddfa640e90/kotlin-scripting-compiler-impl-embeddable-1.9.25.jar
2025-12-03T12:40:06.773-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-native-utils/1.9.25/2fbf43e0ac3289683899732b90a89757cd6fa8f4/kotlin-native-utils-1.9.25.jar
2025-12-03T12:40:06.777-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-util-io/1.9.25/ee4036b501480e4b9fe6a54fd66ae1ea09ff26e5/kotlin-util-io-1.9.25.jar
2025-12-03T12:40:06.779-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-tooling-core/1.9.25/bdc01105250e7ecc8bbcf49a21805685cd557dda/kotlin-tooling-core-1.9.25.jar
2025-12-03T12:40:06.780-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.intellij.deps/trove4j/1.0.20200330/3afb14d5f9ceb459d724e907a21145e8ff394f02/trove4j-1.0.20200330.jar
2025-12-03T12:40:06.776-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'org.gradle.internal.component.external.model.UrlBackedArtifactMetadata@2c97e8bf' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/io.gitlab.arturbosch.detekt/detekt-gradle-plugin/1.23.6/6550d0325c808b81a925e3b4ea4297914fd0a282/detekt-gradle-plugin-1.23.6.jar
2025-12-03T12:40:06.776-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'org.gradle.internal.component.external.model.UrlBackedArtifactMetadata@4de54557' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-gradle-plugin-idea/1.9.25/83cf5d4acdbf6c1e7b32ca96cb79b96e4cce032e/kotlin-gradle-plugin-idea-1.9.25.jar
2025-12-03T12:40:06.780-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-daemon-embeddable/1.9.25/3a1ebb4f98ed480e4774f45e3be215626eb3ae02/kotlin-daemon-embeddable-1.9.25.jar
2025-12-03T12:40:06.779-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-gradle-plugin-annotations/1.9.25/14c3d9e657626be70f8195611976d7bda7148eb8/kotlin-gradle-plugin-annotations-1.9.25.jar
2025-12-03T12:40:06.779-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-daemon-client/1.9.25/31e63adc6d156b785cde045e556814722755c52e/kotlin-daemon-client-1.9.25.jar
2025-12-03T12:40:06.779-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-project-model/1.9.25/9eea9fcc2aa4fef149bd743ab692c29d7f507859/kotlin-project-model-1.9.25.jar
2025-12-03T12:40:06.785-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25)' completed
2025-12-03T12:40:06.783-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'org.gradle.internal.component.external.model.UrlBackedArtifactMetadata@55e44a86' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-core-jvm/1.5.0/d8cebccdcddd029022aa8646a5a953ff88b13ac8/kotlinx-coroutines-core-jvm-1.5.0.jar
2025-12-03T12:40:06.786-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-scripting-jvm/1.9.25/68086b208d6c6bca0daaa3ab51861a2d9c75b6c9/kotlin-scripting-jvm-1.9.25.jar
2025-12-03T12:40:06.786-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: released lock on state of build :
2025-12-03T12:40:06.786-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.ivyresolve.CachingModuleComponentRepository] Found artifact 'kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25)' in resolver cache: /Users/mymac/.gradle/caches/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-scripting-common/1.9.25/91c60ba89e416e5f856c8434ef8cf4f94a7fa5f5/kotlin-scripting-common-1.9.25.jar
2025-12-03T12:40:06.787-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: released lock on worker lease
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: acquired lock on state of build :
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: acquired lock on worker lease
2025-12-03T12:40:06.788-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' completed
2025-12-03T12:40:06.789-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.790-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.790-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.790-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25 variant runtimeElementsWithFixedAttribute doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.791-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25 variant runtimeElementsWithFixedAttribute doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.791-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.792-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.792-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.792-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25 variant gradle82RuntimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.793-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jlleitschuh.gradle:ktlint-gradle:12.1.1 variant runtimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.793-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6 variant runtimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.793-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.794-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.794-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.794-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-native-utils:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.794-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-util-klib:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.794-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.795-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-android-extensions:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.795-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.795-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.795-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.796-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.796-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.796-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-project-model:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.796-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-tooling-core:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.796-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-util-io:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.797-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.797-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.intellij.deps:trove4j:1.0.20200330 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.798-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-daemon-client:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.798-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0 variant jvmRuntimeElements-published doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.799-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.799-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-common:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=instrumented-and-upgraded, org.gradle.jvm.version=17, org.gradle.libraryelements=jar, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.800-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Fingerprint transform inputs' completed
2025-12-03T12:40:06.801-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin.jvm:org.jetbrains.kotlin.jvm.gradle.plugin:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.801-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jlleitschuh.gradle.ktlint:org.jlleitschuh.gradle.ktlint.gradle.plugin:12.1.1 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.802-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate io.gitlab.arturbosch.detekt:io.gitlab.arturbosch.detekt.gradle.plugin:1.23.6 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.802-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25 variant runtimeElementsWithFixedAttribute doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.802-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25 variant runtimeElementsWithFixedAttribute doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.802-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.802-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.803-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugins-bom:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=platform, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.803-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25 variant gradle82RuntimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.804-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jlleitschuh.gradle:ktlint-gradle:12.1.1 variant runtimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.804-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6 variant runtimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.805-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.805-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25 variant runtimeElements doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.805-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.806-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-native-utils:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.806-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-util-klib:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.806-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.806-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-android-extensions:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.807-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.807-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.807-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.807-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.807-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.808-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-project-model:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.808-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-tooling-core:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.808-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-util-io:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.808-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.808-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.intellij.deps:trove4j:1.0.20200330 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.808-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-daemon-client:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.809-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0 variant jvmRuntimeElements-published doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.809-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.809-0600 [DEBUG] [org.gradle.internal.component.model.LoggingAttributeMatchingExplanationBuilder] Candidate org.jetbrains.kotlin:kotlin-scripting-common:1.9.25 configuration runtime doesn't match attributes {org.gradle.category=library, org.gradle.dependency.bundling=external, org.gradle.internal.instrumented=analyzed-artifact, org.gradle.jvm.version=17, org.gradle.libraryelements=classes, org.gradle.plugin.api-version=8.10.2, org.gradle.usage=java-runtime}
2025-12-03T12:40:06.809-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.809-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.810-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.810-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.810-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.810-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.811-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.811-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.811-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.812-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.812-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.812-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.812-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.813-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.813-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.813-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.814-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.814-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.814-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.815-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.815-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.815-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.815-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.816-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.816-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.817-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.817-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.817-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.818-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.818-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.819-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.819-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.819-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.820-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.820-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.821-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.821-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.821-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.821-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.822-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.822-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.822-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.823-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.823-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.823-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.823-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.823-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.824-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.824-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' completed
2025-12-03T12:40:06.825-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Fingerprint transform inputs' completed
2025-12-03T12:40:06.826-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Fingerprint transform inputs' completed
2025-12-03T12:40:06.826-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.826-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.826-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.827-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.828-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.828-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.829-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.829-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.833-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.835-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.835-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.836-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.836-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.837-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.837-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.838-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.839-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.839-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.840-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.840-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-api:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.839-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.840-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.841-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.841-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.840-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.842-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.842-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.843-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.842-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-1.9.25-gradle82.jar (org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.845-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.845-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.844-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.846-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-model:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.845-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.844-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.847-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.848-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.851-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.852-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.852-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.853-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.855-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.855-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.856-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.856-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.856-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming detekt-gradle-plugin-1.23.6.jar (io.gitlab.arturbosch.detekt:detekt-gradle-plugin:1.23.6) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.857-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.857-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.859-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.858-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.861-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.861-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.862-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.863-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming ktlint-gradle-12.1.1.jar (org.jlleitschuh.gradle:ktlint-gradle:12.1.1) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.862-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.866-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.870-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.873-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.877-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.878-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.878-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.878-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.878-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.879-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.880-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.881-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.881-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.882-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.884-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.885-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.886-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.887-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.889-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-proto-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea-proto:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.889-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.891-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.891-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.886-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.890-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.894-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.892-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-native-utils-1.9.25.jar (org.jetbrains.kotlin:kotlin-native-utils:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.897-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.897-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.896-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.898-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.898-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-klib-commonizer-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-klib-commonizer-api:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.897-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.900-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.900-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.899-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.902-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.903-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.904-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.905-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.905-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.906-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.907-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.906-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.907-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.907-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.908-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.908-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.908-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.909-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.909-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-klib-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-klib:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.908-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.913-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.913-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.914-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.914-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.913-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-idea-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-idea:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.915-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.916-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.915-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.917-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.918-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.918-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.919-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.919-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.920-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.919-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.919-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.921-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.923-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.923-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.924-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.924-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.923-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.923-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.925-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.926-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.926-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-android-extensions-1.9.25.jar (org.jetbrains.kotlin:kotlin-android-extensions:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.927-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.928-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.928-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.928-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.928-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.929-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.929-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.928-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.930-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.931-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.932-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.932-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.933-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.933-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.933-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.932-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-runner-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-runner:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.933-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.933-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-build-tools-api-1.9.25.jar (org.jetbrains.kotlin:kotlin-build-tools-api:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.936-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.936-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.936-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.936-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-compiler-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.937-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.938-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.938-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.939-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.940-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.939-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.940-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.940-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.941-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.942-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.942-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.943-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.943-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.944-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.945-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.946-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.944-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.947-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.947-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.946-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.948-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-impl-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-impl-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.950-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.951-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.952-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.953-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.954-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.955-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.955-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.955-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.956-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.955-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.956-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-gradle-plugin-annotations-1.9.25.jar (org.jetbrains.kotlin:kotlin-gradle-plugin-annotations:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.957-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.958-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-compiler-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-compiler-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.959-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.960-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.960-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.961-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.962-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.963-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.964-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.964-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.964-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-tooling-core-1.9.25.jar (org.jetbrains.kotlin:kotlin-tooling-core:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.967-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.968-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.968-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.974-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.976-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.977-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.977-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.979-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.979-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-project-model-1.9.25.jar (org.jetbrains.kotlin:kotlin-project-model:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:06.979-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.979-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.980-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.989-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.989-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.991-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.991-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.991-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.994-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.994-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.994-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.995-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.995-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.995-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.996-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.997-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:06.997-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:06.998-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:06.998-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:07.005-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with InstrumentationAnalysisTransform
2025-12-03T12:40:07.010-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.016-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with MergeInstrumentationAnalysisTransform
2025-12-03T12:40:07.026-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.029-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.038-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.040-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.041-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.042-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.042-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.042-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.042-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlinx-coroutines-core-jvm-1.5.0.jar (org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm:1.5.0) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:07.044-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.047-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.048-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.048-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.049-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.050-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.051-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.047-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.054-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.055-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.055-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.055-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-util-io-1.9.25.jar (org.jetbrains.kotlin:kotlin-util-io:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:07.056-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.056-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.057-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.052-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.049-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-jvm-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-jvm:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:07.065-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.070-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.073-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.078-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.082-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.086-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.086-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming trove4j-1.0.20200330.jar (org.jetbrains.intellij.deps:trove4j:1.0.20200330) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:07.088-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.089-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.090-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.090-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.092-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.093-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.097-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.097-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.098-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.099-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.097-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-client-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-client:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:07.099-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-daemon-embeddable-1.9.25.jar (org.jetbrains.kotlin:kotlin-daemon-embeddable:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:07.099-0600 [INFO] [org.gradle.api.internal.artifacts.transform.TransformStep] Transforming kotlin-scripting-common-1.9.25.jar (org.jetbrains.kotlin:kotlin-scripting-common:1.9.25) with ExternalDependencyInstrumentingArtifactTransform
2025-12-03T12:40:07.100-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.101-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.101-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.105-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.105-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.105-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.107-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.326-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute transform' completed
2025-12-03T12:40:07.326-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.326-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: released lock on state of build :
2025-12-03T12:40:07.326-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: released lock on worker lease
2025-12-03T12:40:07.327-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.328-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: acquired lock on state of build :
2025-12-03T12:40:07.328-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: acquired lock on worker lease
2025-12-03T12:40:07.329-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' completed
2025-12-03T12:40:07.329-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Resolve files of configuration ':classpath'' completed
2025-12-03T12:40:07.331-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.kotlin.dsl.provider.plugins.KotlinScriptRootPlugin to root project 'ron-app-sdk-jvm'' completed
2025-12-03T12:40:07.332-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Register task :kotlinDslAccessorsReport' completed
2025-12-03T12:40:07.332-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply plugin org.gradle.kotlin.dsl.provider.plugins.KotlinScriptBasePlugin to root project 'ron-app-sdk-jvm'' completed
2025-12-03T12:40:07.373-0600 [DEBUG] [org.gradle.api.internal.runtimeshaded.RuntimeShadedJarFactory] Using Gradle runtime shaded JAR file: /Users/mymac/.gradle/caches/8.10.2/generated-gradle-jars/gradle-api-8.10.2.jar
2025-12-03T12:40:07.385-0600 [DEBUG] [org.gradle.internal.locking.LockFileReaderWriter] Lockfiles root: /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/gradle/dependency-locks
2025-12-03T12:40:07.387-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Identifying work' completed
2025-12-03T12:40:07.389-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Cross-configure project :' completed
2025-12-03T12:40:07.389-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute 'allprojects {}' action' completed
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Execute 'subprojects {}' action' completed
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Apply build file 'build.gradle.kts' to root project 'ron-app-sdk-jvm'' completed
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.configuration.project.BuildScriptProcessor] Timing: Running the build script took 0.803 secs
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify afterEvaluate listeners of :' completed
2025-12-03T12:40:07.390-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: released lock on state of build :
2025-12-03T12:40:07.391-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Configure project :'
2025-12-03T12:40:07.391-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Configure project :' completed
2025-12-03T12:40:07.391-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify projectsEvaluated listeners' started
2025-12-03T12:40:07.391-0600 [INFO] [org.gradle.internal.buildevents.BuildLogger] All projects evaluated.
2025-12-03T12:40:07.391-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Notify projectsEvaluated listeners'
2025-12-03T12:40:07.391-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Notify projectsEvaluated listeners' completed
2025-12-03T12:40:07.391-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Configure build'
2025-12-03T12:40:07.392-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Configure build' completed
2025-12-03T12:40:07.392-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Calculate build tree task graph' started
2025-12-03T12:40:07.392-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Calculate task graph' started
2025-12-03T12:40:07.392-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: acquired lock on All projects of :
2025-12-03T12:40:07.393-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: released lock on All projects of :
2025-12-03T12:40:07.393-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Calculate task graph'
2025-12-03T12:40:07.393-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Calculate task graph' completed
2025-12-03T12:40:07.393-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Calculate build tree task graph'
2025-12-03T12:40:07.393-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Calculate build tree task graph' completed
2025-12-03T12:40:07.394-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Finish root build tree' started
2025-12-03T12:40:07.395-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Finish root build tree'
2025-12-03T12:40:07.395-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Finish root build tree' completed
2025-12-03T12:40:07.396-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] 
2025-12-03T12:40:07.399-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] FAILURE: Build failed with an exception.
2025-12-03T12:40:07.399-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] 
2025-12-03T12:40:07.399-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] * What went wrong:
2025-12-03T12:40:07.400-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] Cannot locate tasks that match ':core:test' as project 'core' not found in root project 'ron-app-sdk-jvm'.
2025-12-03T12:40:07.400-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] 
2025-12-03T12:40:07.400-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] * Try:
2025-12-03T12:40:07.400-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] > Run gradlew projects to get a list of available projects.
2025-12-03T12:40:07.400-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] > For more on name expansion, please refer to https://docs.gradle.org/8.10.2/userguide/command_line_interface.html#sec:name_abbreviation in the Gradle documentation.
2025-12-03T12:40:07.401-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] > Run with --scan to get full insights.
2025-12-03T12:40:07.401-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] > Get more help at https://help.gradle.org.
2025-12-03T12:40:07.401-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] 
2025-12-03T12:40:07.401-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] * Exception is:
2025-12-03T12:40:07.402-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] org.gradle.execution.ProjectSelectionException: Cannot locate tasks that match ':core:test' as project 'core' not found in root project 'ron-app-sdk-jvm'.
2025-12-03T12:40:07.402-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.execution.selection.DefaultBuildTaskSelector.lambda$selectProject$2(DefaultBuildTaskSelector.java:180)
2025-12-03T12:40:07.402-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.api.problems.internal.DefaultProblemReporter.throwing(DefaultProblemReporter.java:57)
2025-12-03T12:40:07.402-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.execution.selection.DefaultBuildTaskSelector.selectProject(DefaultBuildTaskSelector.java:177)
2025-12-03T12:40:07.403-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.execution.selection.DefaultBuildTaskSelector.resolveProject(DefaultBuildTaskSelector.java:135)
2025-12-03T12:40:07.404-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.execution.selection.DefaultBuildTaskSelector.resolveTaskName(DefaultBuildTaskSelector.java:107)
2025-12-03T12:40:07.404-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.execution.commandline.CommandLineTaskParser.parseTasks(CommandLineTaskParser.java:49)
2025-12-03T12:40:07.404-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.execution.TaskNameResolvingBuildTaskScheduler.scheduleRequestedTasks(TaskNameResolvingBuildTaskScheduler.java:51)
2025-12-03T12:40:07.404-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.execution.DefaultTasksBuildTaskScheduler.scheduleRequestedTasks(DefaultTasksBuildTaskScheduler.java:72)
2025-12-03T12:40:07.405-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.initialization.DefaultTaskExecutionPreparer.lambda$scheduleRequestedTasks$0(DefaultTaskExecutionPreparer.java:46)
2025-12-03T12:40:07.405-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.Factories$1.create(Factories.java:31)
2025-12-03T12:40:07.405-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.work.DefaultWorkerLeaseService.withReplacedLocks(DefaultWorkerLeaseService.java:359)
2025-12-03T12:40:07.405-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.api.internal.project.DefaultProjectStateRegistry$DefaultBuildProjectRegistry.withMutableStateOfAllProjects(DefaultProjectStateRegistry.java:238)
2025-12-03T12:40:07.405-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.api.internal.project.DefaultProjectStateRegistry$DefaultBuildProjectRegistry.withMutableStateOfAllProjects(DefaultProjectStateRegistry.java:231)
2025-12-03T12:40:07.406-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.initialization.DefaultTaskExecutionPreparer.scheduleRequestedTasks(DefaultTaskExecutionPreparer.java:45)
2025-12-03T12:40:07.406-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.initialization.VintageBuildModelController.lambda$scheduleRequestedTasks$0(VintageBuildModelController.java:76)
2025-12-03T12:40:07.406-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.lambda$inState$1(StateTransitionController.java:99)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.lambda$inState$2(StateTransitionController.java:114)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.work.DefaultSynchronizer.withLock(DefaultSynchronizer.java:44)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:110)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:98)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.initialization.VintageBuildModelController.scheduleRequestedTasks(VintageBuildModelController.java:76)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.DefaultBuildLifecycleController$DefaultWorkGraphBuilder.addRequestedTasks(DefaultBuildLifecycleController.java:404)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeWorkPreparer.lambda$scheduleRequestedTasks$0(DefaultBuildTreeWorkPreparer.java:41)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.DefaultBuildLifecycleController.lambda$populateWorkGraph$7(DefaultBuildLifecycleController.java:189)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.DefaultBuildWorkPreparer.populateWorkGraph(DefaultBuildWorkPreparer.java:42)
2025-12-03T12:40:07.407-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.BuildOperationFiringBuildWorkPreparer$PopulateWorkGraph.populateTaskGraph(BuildOperationFiringBuildWorkPreparer.java:106)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.BuildOperationFiringBuildWorkPreparer$PopulateWorkGraph.run(BuildOperationFiringBuildWorkPreparer.java:92)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:29)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:26)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:66)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:59)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:166)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:59)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.run(DefaultBuildOperationRunner.java:47)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.BuildOperationFiringBuildWorkPreparer.populateWorkGraph(BuildOperationFiringBuildWorkPreparer.java:67)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.DefaultBuildLifecycleController.lambda$populateWorkGraph$8(DefaultBuildLifecycleController.java:189)
2025-12-03T12:40:07.408-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.lambda$inState$1(StateTransitionController.java:99)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.lambda$inState$2(StateTransitionController.java:114)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.work.DefaultSynchronizer.withLock(DefaultSynchronizer.java:44)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:110)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.inState(StateTransitionController.java:98)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.DefaultBuildLifecycleController.populateWorkGraph(DefaultBuildLifecycleController.java:189)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.build.DefaultBuildWorkGraphController$DefaultBuildWorkGraph.populateWorkGraph(DefaultBuildWorkGraphController.java:169)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.composite.internal.DefaultBuildController.populateWorkGraph(DefaultBuildController.java:76)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph$DefaultBuildTreeWorkGraphBuilder.withWorkGraph(DefaultIncludedBuildTaskGraph.java:153)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeWorkPreparer.lambda$scheduleRequestedTasks$1(DefaultBuildTreeWorkPreparer.java:41)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph$DefaultBuildTreeWorkGraph$1.run(DefaultIncludedBuildTaskGraph.java:209)
2025-12-03T12:40:07.409-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:29)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$1.execute(DefaultBuildOperationRunner.java:26)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:66)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:59)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:166)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:59)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.run(DefaultBuildOperationRunner.java:47)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph$DefaultBuildTreeWorkGraph.scheduleWork(DefaultIncludedBuildTaskGraph.java:204)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeWorkPreparer.scheduleRequestedTasks(DefaultBuildTreeWorkPreparer.java:37)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.cc.impl.VintageBuildTreeWorkController$scheduleAndRunRequestedTasks$1.apply(VintageBuildTreeWorkController.kt:36)
2025-12-03T12:40:07.410-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.cc.impl.VintageBuildTreeWorkController$scheduleAndRunRequestedTasks$1.apply(VintageBuildTreeWorkController.kt:35)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.composite.internal.DefaultIncludedBuildTaskGraph.withNewWorkGraph(DefaultIncludedBuildTaskGraph.java:112)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.cc.impl.VintageBuildTreeWorkController.scheduleAndRunRequestedTasks(VintageBuildTreeWorkController.kt:35)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.lambda$scheduleAndRunTasks$1(DefaultBuildTreeLifecycleController.java:77)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.lambda$runBuild$4(DefaultBuildTreeLifecycleController.java:120)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.lambda$transition$6(StateTransitionController.java:169)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.doTransition(StateTransitionController.java:266)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.lambda$transition$7(StateTransitionController.java:169)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.work.DefaultSynchronizer.withLock(DefaultSynchronizer.java:44)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.model.StateTransitionController.transition(StateTransitionController.java:169)
2025-12-03T12:40:07.411-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.runBuild(DefaultBuildTreeLifecycleController.java:117)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.scheduleAndRunTasks(DefaultBuildTreeLifecycleController.java:77)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeLifecycleController.scheduleAndRunTasks(DefaultBuildTreeLifecycleController.java:72)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.tooling.internal.provider.ExecuteBuildActionRunner.run(ExecuteBuildActionRunner.java:31)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.ChainingBuildActionRunner.run(ChainingBuildActionRunner.java:35)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.ProblemReportingBuildActionRunner.run(ProblemReportingBuildActionRunner.java:49)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.BuildOutcomeReportingBuildActionRunner.run(BuildOutcomeReportingBuildActionRunner.java:65)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.tooling.internal.provider.FileSystemWatchingBuildActionRunner.run(FileSystemWatchingBuildActionRunner.java:140)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.BuildCompletionNotifyingBuildActionRunner.run(BuildCompletionNotifyingBuildActionRunner.java:41)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.RootBuildLifecycleBuildActionExecutor.lambda$execute$0(RootBuildLifecycleBuildActionExecutor.java:54)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.composite.internal.DefaultRootBuildState.run(DefaultRootBuildState.java:130)
2025-12-03T12:40:07.412-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.RootBuildLifecycleBuildActionExecutor.execute(RootBuildLifecycleBuildActionExecutor.java:54)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.InitDeprecationLoggingActionExecutor.execute(InitDeprecationLoggingActionExecutor.java:62)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.InitProblems.execute(InitProblems.java:36)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.DefaultBuildTreeContext.execute(DefaultBuildTreeContext.java:40)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.BuildTreeLifecycleBuildActionExecutor.lambda$execute$0(BuildTreeLifecycleBuildActionExecutor.java:71)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildtree.BuildTreeState.run(BuildTreeState.java:60)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.BuildTreeLifecycleBuildActionExecutor.execute(BuildTreeLifecycleBuildActionExecutor.java:71)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.RunAsBuildOperationBuildActionExecutor$3.call(RunAsBuildOperationBuildActionExecutor.java:61)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.RunAsBuildOperationBuildActionExecutor$3.call(RunAsBuildOperationBuildActionExecutor.java:57)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$CallableBuildOperationWorker.execute(DefaultBuildOperationRunner.java:209)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$CallableBuildOperationWorker.execute(DefaultBuildOperationRunner.java:204)
2025-12-03T12:40:07.413-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:66)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner$2.execute(DefaultBuildOperationRunner.java:59)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:166)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.execute(DefaultBuildOperationRunner.java:59)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.operations.DefaultBuildOperationRunner.call(DefaultBuildOperationRunner.java:53)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.RunAsBuildOperationBuildActionExecutor.execute(RunAsBuildOperationBuildActionExecutor.java:57)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.RunAsWorkerThreadBuildActionExecutor.lambda$execute$0(RunAsWorkerThreadBuildActionExecutor.java:36)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.work.DefaultWorkerLeaseService.withLocks(DefaultWorkerLeaseService.java:263)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.work.DefaultWorkerLeaseService.runAsWorkerThread(DefaultWorkerLeaseService.java:127)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.exec.RunAsWorkerThreadBuildActionExecutor.execute(RunAsWorkerThreadBuildActionExecutor.java:36)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.tooling.internal.provider.continuous.ContinuousBuildActionExecutor.execute(ContinuousBuildActionExecutor.java:110)
2025-12-03T12:40:07.414-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.tooling.internal.provider.SubscribableBuildActionExecutor.execute(SubscribableBuildActionExecutor.java:64)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.session.DefaultBuildSessionContext.execute(DefaultBuildSessionContext.java:46)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor$ActionImpl.apply(BuildSessionLifecycleBuildActionExecutor.java:92)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor$ActionImpl.apply(BuildSessionLifecycleBuildActionExecutor.java:80)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.session.BuildSessionState.run(BuildSessionState.java:71)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor.execute(BuildSessionLifecycleBuildActionExecutor.java:62)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.BuildSessionLifecycleBuildActionExecutor.execute(BuildSessionLifecycleBuildActionExecutor.java:41)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.StartParamsValidatingActionExecutor.execute(StartParamsValidatingActionExecutor.java:64)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.StartParamsValidatingActionExecutor.execute(StartParamsValidatingActionExecutor.java:32)
2025-12-03T12:40:07.415-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.SessionFailureReportingActionExecutor.execute(SessionFailureReportingActionExecutor.java:51)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.SessionFailureReportingActionExecutor.execute(SessionFailureReportingActionExecutor.java:39)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.SetupLoggingActionExecutor.execute(SetupLoggingActionExecutor.java:47)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.buildprocess.execution.SetupLoggingActionExecutor.execute(SetupLoggingActionExecutor.java:31)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.ExecuteBuild.doBuild(ExecuteBuild.java:70)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.BuildCommandOnly.execute(BuildCommandOnly.java:37)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.WatchForDisconnection.execute(WatchForDisconnection.java:39)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.416-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.ResetDeprecationLogger.execute(ResetDeprecationLogger.java:29)
2025-12-03T12:40:07.417-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.417-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.RequestStopIfSingleUsedDaemon.execute(RequestStopIfSingleUsedDaemon.java:35)
2025-12-03T12:40:07.417-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.417-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.ForwardClientInput.lambda$execute$0(ForwardClientInput.java:40)
2025-12-03T12:40:07.417-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.daemon.clientinput.ClientInputForwarder.forwardInput(ClientInputForwarder.java:80)
2025-12-03T12:40:07.417-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.ForwardClientInput.execute(ForwardClientInput.java:37)
2025-12-03T12:40:07.417-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.LogAndCheckHealth.execute(LogAndCheckHealth.java:64)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.LogToClient.doBuild(LogToClient.java:63)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.BuildCommandOnly.execute(BuildCommandOnly.java:37)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.EstablishBuildEnvironment.doBuild(EstablishBuildEnvironment.java:84)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.BuildCommandOnly.execute(BuildCommandOnly.java:37)
2025-12-03T12:40:07.418-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.api.DaemonCommandExecution.proceed(DaemonCommandExecution.java:104)
2025-12-03T12:40:07.419-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.exec.StartBuildOrRespondWithBusy$1.run(StartBuildOrRespondWithBusy.java:52)
2025-12-03T12:40:07.419-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.launcher.daemon.server.DaemonStateCoordinator.lambda$runCommand$0(DaemonStateCoordinator.java:321)
2025-12-03T12:40:07.419-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.concurrent.ExecutorPolicy$CatchAndRecordFailures.onExecute(ExecutorPolicy.java:64)
2025-12-03T12:40:07.419-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter]   at org.gradle.internal.concurrent.AbstractManagedExecutor$1.run(AbstractManagedExecutor.java:48)
2025-12-03T12:40:07.419-0600 [ERROR] [org.gradle.internal.buildevents.BuildExceptionReporter] 
2025-12-03T12:40:07.419-0600 [ERROR] [org.gradle.internal.buildevents.BuildResultLogger] 
2025-12-03T12:40:07.419-0600 [ERROR] [org.gradle.internal.buildevents.BuildResultLogger] BUILD FAILED in 2s
2025-12-03T12:40:07.419-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Build finished for file system watching' started
2025-12-03T12:40:07.421-0600 [INFO] [org.gradle.internal.watch.registry.impl.HierarchicalFileWatcherUpdater] Watched directory hierarchies: [/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm]
2025-12-03T12:40:07.421-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Build finished for file system watching'
2025-12-03T12:40:07.421-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Build finished for file system watching' completed
2025-12-03T12:40:07.422-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Releasing file lock for cache directory md-supplier (/Users/mymac/.gradle/caches/8.10.2/md-supplier)
2025-12-03T12:40:07.422-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Releasing lock on cache directory md-supplier (/Users/mymac/.gradle/caches/8.10.2/md-supplier).
2025-12-03T12:40:07.423-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Releasing file lock for Build Output Cleanup Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/buildOutputCleanup)
2025-12-03T12:40:07.423-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Releasing lock on Build Output Cleanup Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/buildOutputCleanup).
2025-12-03T12:40:07.424-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Releasing file lock for cache directory md-rule (/Users/mymac/.gradle/caches/8.10.2/md-rule)
2025-12-03T12:40:07.424-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Releasing lock on cache directory md-rule (/Users/mymac/.gradle/caches/8.10.2/md-rule).
2025-12-03T12:40:07.425-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.store.CachedStoreFactory] Resolution result cache closed. Cache reads: 0, disk reads: 0 (avg: 0.0 secs, total: 0.0 secs)
2025-12-03T12:40:07.425-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.store.CachedStoreFactory] Resolution result cache closed. Cache reads: 0, disk reads: 0 (avg: 0.0 secs, total: 0.0 secs)
2025-12-03T12:40:07.425-0600 [DEBUG] [org.gradle.api.internal.artifacts.ivyservice.resolveengine.store.ResolutionResultsStoreFactory] Deleted 2 resolution results binary files in 0.0 secs
2025-12-03T12:40:07.425-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Run build'
2025-12-03T12:40:07.426-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Run build' completed
2025-12-03T12:40:07.426-0600 [DEBUG] [org.gradle.internal.resources.AbstractTrackedResourceLock] Daemon worker Thread 15: released lock on worker lease
2025-12-03T12:40:07.437-0600 [DEBUG] [org.gradle.deployment.internal.DefaultDeploymentRegistry] Stopping 0 deployment handles
2025-12-03T12:40:07.437-0600 [DEBUG] [org.gradle.deployment.internal.DefaultDeploymentRegistry] Stopped deployment handles
2025-12-03T12:40:07.438-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Releasing file lock for file hash cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/fileHashes)
2025-12-03T12:40:07.438-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Releasing lock on file hash cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/fileHashes).
2025-12-03T12:40:07.438-0600 [DEBUG] [org.gradle.cache.internal.LockOnDemandCrossProcessCacheAccess] Releasing file lock for checksums cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums)
2025-12-03T12:40:07.439-0600 [DEBUG] [org.gradle.cache.internal.DefaultFileLockManager] Releasing lock on checksums cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/checksums).
2025-12-03T12:40:07.439-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCleanupExecutor] VCS Checkout Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/vcs-1) has last been fully cleaned up 1 hours ago
2025-12-03T12:40:07.439-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCleanupExecutor] Skipping cleanup for VCS Checkout Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/vcs-1) as it is not yet due
2025-12-03T12:40:07.439-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Cache VCS Checkout Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/vcs-1) was closed 0 times.
2025-12-03T12:40:07.439-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Cache VCS metadata (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/vcsMetadata) was closed 0 times.
2025-12-03T12:40:07.439-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCleanupExecutor] dependencies-accessors (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/dependencies-accessors) has last been fully cleaned up 1 hours ago
2025-12-03T12:40:07.440-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCleanupExecutor] Skipping cleanup for dependencies-accessors (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/dependencies-accessors) as it is not yet due
2025-12-03T12:40:07.440-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Cache dependencies-accessors (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/dependencies-accessors) was closed 0 times.
2025-12-03T12:40:07.440-0600 [DEBUG] [org.gradle.cache.internal.DefaultCacheCoordinator] Cache Compressed Files Expansion Cache (/Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle/8.10.2/expanded) was closed 0 times.
2025-12-03T12:40:07.440-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Deleting unused version-specific caches in /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle' started
2025-12-03T12:40:07.440-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Completing Build operation 'Deleting unused version-specific caches in /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle'
2025-12-03T12:40:07.440-0600 [DEBUG] [org.gradle.internal.operations.DefaultBuildOperationRunner] Build operation 'Deleting unused version-specific caches in /Users/mymac/Desktop/RustyOnions/sdk/ron-app-sdk-jvm/.gradle' completed
2025-12-03T12:40:07.448-0600 [DEBUG] [org.gradle.launcher.daemon.server.exec.ExecuteBuild] The daemon has finished executing the build.
2025-12-03T12:40:07.684-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClientConnection] thread 1: dispatching class org.gradle.launcher.daemon.protocol.CloseInput
2025-12-03T12:40:07.684-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClient] Received result Success[value=org.gradle.launcher.exec.BuildActionResult@67427b69] from daemon DaemonInfo{pid=1456, address=[7109e125-342f-4ec3-b525-9f055a61bcb3 port:49307, addresses:[/127.0.0.1]], state=Idle, lastBusy=1764787200080, context=DefaultDaemonContext[uid=6a7cdb50-ad0f-4d8a-bccb-46fc82be61f9,javaHome=/Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home,javaVersion=17,javaVendor=Eclipse Adoptium,daemonRegistryDir=/Users/mymac/.gradle/daemon,pid=1456,idleTimeout=10800000,priority=NORMAL,applyInstrumentationAgent=true,nativeServicesMode=ENABLED,daemonOpts=--add-opens=java.base/java.util=ALL-UNNAMED,--add-opens=java.base/java.lang=ALL-UNNAMED,--add-opens=java.base/java.lang.invoke=ALL-UNNAMED,--add-opens=java.prefs/java.util.prefs=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED,--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED,--add-opens=java.base/java.nio.charset=ALL-UNNAMED,--add-opens=java.base/java.net=ALL-UNNAMED,--add-opens=java.base/java.util.concurrent.atomic=ALL-UNNAMED,-XX:MaxMetaspaceSize=384m,-XX:+HeapDumpOnOutOfMemoryError,-Xms256m,-Xmx512m,-Dfile.encoding=UTF-8,-Duser.country=US,-Duser.language=en,-Duser.variant]} (build should be done).
2025-12-03T12:40:07.685-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClientConnection] thread 1: dispatching class org.gradle.launcher.daemon.protocol.Finished
2025-12-03T12:40:07.685-0600 [DEBUG] [org.gradle.launcher.daemon.client.DaemonClientConnection] thread 1: connection stop
2025-12-03T12:40:07.734-0600 [LIFECYCLE] [org.gradle.launcher.cli.DebugLoggerWarningAction] 
#############################################################################
   WARNING WARNING WARNING WARNING WARNING WARNING WARNING WARNING WARNING

   Debug level logging will leak security sensitive information!

   For more details, please refer to https://docs.gradle.org/8.10.2/userguide/logging.html#sec:debug_security in the Gradle documentation.
#############################################################################

mymac@RustyOnionsDevNet ron-app-sdk-jvm % 

