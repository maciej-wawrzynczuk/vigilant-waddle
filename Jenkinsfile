pipeline {
    agent { label 'rust' }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
            }
        }

        stage('Test') {
            steps {
                sh '''
                    . $HOME/.cargo/env
                    cargo test
                '''
            }
        }
        stage('Build bin') {
            steps {
                sh '''
                    cargo build --bin waddle-ws --target=x86_64-unknown-linux-musl
                '''
            }
        }
        stage('Build image') {
            steps {
                script {
                    COMMIT_HASH = sh(
                        script: "git rev-parse --short HEAD",
                        returnStdout: true
                    )
                    sh """
                        buildah bud -t waddle-ws:latest -t waddle-ws:${COMMIT_HASH} .
                    """
                }
            }
        }
    }
}
