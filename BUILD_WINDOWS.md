# Windows Build Guide

## Prerequisites

### 1. Java Development Kit (JDK) 25

### 2. Apache Maven 3.9+

### 3. Node.js 20+

### 4. CMake 3.16+
- Download and install [CMake](https://cmake.org/download/) (version 3.16+)
- During installation, select "Add CMake to system PATH"

### 5. C++ Compiler (choose one)

#### Option A: Visual Studio 2022 (Recommended)
- Download [Visual Studio Community 2026](https://visualstudio.microsoft.com/downloads/)
- During installation, select "Desktop development with C++"
- Includes MSVC compiler and build tools

#### Option B: MinGW-w64
- Install via [MSYS2](https://www.msys2.org/):
  ```bash
  pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-cmake
  ```
- Add MinGW `bin` directory to PATH

## Build Order

The components must be built in this order due to dependencies:

1. **Proto** (generates shared protocol definitions)
2. **Markdown Parser** (C++ library)
3. **Journal Server** (depends on Proto)
4. **Frontend** (depends on Proto)

## Building the Project

`peta-journal` is the project root directory.

### 1. Build Proto Definitions

```cmd
cd peta-journal/proto
mvn clean install
```

This installs the proto JAR to your local Maven repository for use by journal-server.

### 2. Build Markdown Parser

#### Using Visual Studio (MSVC)

If you haven't added cmake to your PATH, please run it from Developer Command Prompt or Developer PowerShell.

```cmd
cd peta-journal/markdown-parser
mkdir build
cd build
cmake .. -G "Visual Studio 18 2026" -A x64
cmake --build . --config Release
```

To run tests:
```powershell
ctest -C Release

#### Using MinGW

```powershell
cd peta-journal/markdown-parser
mkdir build
cd build
cmake .. -G "Ninja" -DCMAKE_BUILD_TYPE=Release
cmake --build .
```

To run tests:
```powershell
ctest -C Release
```

#### Copy DLL to Journal Server

The journal-server needs the markdown parser DLL. Copy it to the correct location:

**For MSVC:**
```powershell
Copy-Item Release\peta_markdown.dll ..\..\journal-server\lib\windows\libpeta_markdown.dll
```

**For MinGW:**
```powershell
Copy-Item libpeta_markdown.dll ..\..\journal-server\lib\windows\libpeta_markdown.dll
```

### 3. Build Journal Server

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

### 4. Build Frontend

Install dependencies:
```powershell
cd peta-journal/frontend
npm i --legacy-peer-deps
```
We must use the --legacy-peer-deps option due to a dependency conflict between protobufjs@8 and protobufjs-cli@7. This is patched in https://github.com/protobufjs/protobuf.js/pull/2118 but not yet released as of 2026-02-08.

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

## Building All Components (PowerShell Script)

See `build-all.ps1` in the root directory for an automated build script.

