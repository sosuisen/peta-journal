# PetaJournal Build Guide

## Prerequisites

### 1. Java Development Kit (JDK) 25

### 2. Apache Maven 3.9+

### 3. Node.js 20+

The flatmarkdown library (`com.sosuisha:flatmarkdown`) comes from Maven Central and
bundles its native binaries for Windows x86-64 and macOS arm64, so there is nothing to
build by hand.

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

## Development Workflow

1. Start journal server: `cd journal-server && mvn quarkus:dev`
2. Start frontend: `cd frontend && npm start`
3. Access application at `http://localhost:4200`

