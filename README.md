# Dot Viewer 📊

## Summary of Project
Dot Viewer is a native macOS application that enables users to view and edit Graphviz `.dot` files, featuring a split-pane interface with a text editor on the left and a live SVG preview on the right. The app supports tabbed multi-file editing using the native macOS window tabs, providing a seamless experience for users working with graphical representations.

**Notable Features:**
- **Live SVG Preview:** Automatically render and display changes as you edit.
- **Bidirectional Linking:** Click a node in the preview to jump to its definition in the editor; place your cursor on a node to highlight it in the preview.
- **Line Numbers:** Gutter with dynamic-width line numbers, current line highlighting, and click-to-select.
- **Syntax Highlighting:** Visual cues for DOT keywords, strings, comments, attributes, and arrow operators.
- **Bracket Matching:** Matching brackets are highlighted as you navigate the code.
- **Multiple Layout Engines:** Switch between dot, neato, fdp, circo, twopi, and sfdp.
- **Undo/Redo Support:** Easily revert changes with built-in undo functionality.
- **Auto-Updates:** Securely packaged for distribution via Sparkle.
- **Tabbed Editing:** Open multiple `.dot` files as tabs in a single window.

## How to Use
### Prerequisites
- macOS 14.0 or later
- Xcode 16.0 or later
- Homebrew installed on your macOS for dependencies

### Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/2389-research/dot-viewer.git
   cd dot-viewer
   ```

2. Ensure you have the required packages by running:
   ```bash
   brew install xcodegen bison flex
   ```

3. Clone the Graphviz source (vendored dependency):
   ```bash
   cd dot-core && git clone --depth 1 --branch 12.2.1 https://gitlab.com/graphviz/graphviz.git graphviz-vendor && cd ..
   ```

4. Build the app:
   ```bash
   make
   ```

5. Open the project in Xcode:
   ```bash
   open DotViewer/DotViewer.xcodeproj
   ```

6. Run the app through Xcode to launch the Dot Viewer.

### Running Tests
```bash
# Unit tests (DotParser logic)
xcodebuild test -scheme DotViewer -configuration Debug -destination 'platform=macOS' -only-testing:DotViewerTests

# UI scenario tests (requires macOS automation permission)
xcodebuild test -scheme DotViewer -configuration Debug -destination 'platform=macOS' -only-testing:DotViewerUITests
```

### Usage
- Open or create a `.dot` file through the app.
- Use the editor on the left side to modify the DOT input.
- The right side will automatically reflect changes as a live SVG.
- Use the toolbar for toggling live editing, selecting layout engines, and refreshing the preview.

## Tech Info
- **Tech Stack:** 
  - Rust for the core processing 🔧
  - SwiftUI for the user interface 🌟
  - Graphviz for rendering graphs 🎭
  - GitHub Actions for CI/CD workflow 🤖

- **Directory Structure:**
  ```plaintext
  .
  ├── .github/                     # GitHub workflows
  ├── docs/                        # Documentation files
  ├── dot-core/                    # Rust core library
  ├── DotViewer/                   # SwiftUI macOS app source
  ├── Makefile                     # Builds and manages the project
  ├── scripts/                     # Helper scripts
  ├── .gitignore                   # Files to ignore in git
  └── README.md                    # This file
  ```

For detailed setup, workflow, and implementation notes, check the [documentation files](./docs) in this repository. If you encounter any issues, please raise them in the Issues section of this repository or create a pull request with your suggestions for improvements.

**Let's view the world of graphs! 🌍**
