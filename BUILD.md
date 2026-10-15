# PetaJournal Build Guide

## Prerequisites

### 1. Java Development Kit (JDK) 25

### 2. Apache Maven 3.9+

### 3. Node.js 20+

The flatmarkdown library (`com.sosuisha:flatmarkdown`) comes from Maven Central and
bundles its native binaries for Linux x86-64, Windows x86-64 and macOS arm64, so there
is nothing to build by hand.

## Build

### 1. Build Journal Server

```powershell
cd peta-journal/journal-server
mvn clean package
```

This creates a Quarkus application in `target/quarkus-app/`.

To run in development mode:
```powershell
mvn quarkus:dev
```

To run the packaged application:
```cmd
java -jar target\quarkus-app\quarkus-run.jar
```

### 2. Build Frontend

Install dependencies:
```powershell
cd peta-journal/frontend
npm i
```

Run development server:
```powershell
npm start
```

Build for production:
```powershell
npm run build
```

The built files will be in `frontend\dist\`.

### 3. End-to-end tests

The end-to-end tests drive the real app with Playwright: the journal server on port 8187
with a throw-away data directory, and the Angular dev server on port 4300 in front of it.

```powershell
cd peta-journal/journal-server
mvn package -DskipTests
cd ../frontend
npx playwright install chromium   # once
npm run e2e
```

### 4. Server integration tests

`mvn test` in `journal-server` runs the unit tests and the integration tests (`*ApiTest`,
`FolderSyncTest`) together. The integration tests start the server once with its database,
search index, markdown folder and config file under a temporary directory, so they never
touch `~/.peta-journal`.

## Development Workflow

1. Start journal server: `cd journal-server && mvn quarkus:dev`
2. Start frontend: `cd frontend && npm start`
3. Access application at `http://localhost:4200`

