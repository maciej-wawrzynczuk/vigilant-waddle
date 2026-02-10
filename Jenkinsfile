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
    }
}
