# Dot Viewer 📊

## Summary of Project
Dot Viewer is a native macOS application that enables users to view and edit Graphviz `.dot` files, featuring a split-pane interface with a text editor on the left and a live SVG preview on the right. The app supports tabbed multi-file editing using the native macOS window tabs, providing a seamless experience for users working with graphical representations.

**Notable Features:**
- **Live SVG Preview:** Automatically render and display changes as you edit.
- **Syntax Highlighting:** Enhanced editing experience with visual cues for DOT language syntax.
- **Undo/Redo Support:** Easily revert changes with built-in undo functionality.
- **Code Signing & Notarization:** The app is securely packaged for distribution via Sparkle, ensuring users can install updates effortlessly.
- **Intuitive Interface:** Splitting the UI between code and visual output to facilitate understanding and editing of graph structures.

## How to Use
### Prerequisites
- macOS 14.0 or later
- Xcode (version 16.0 or later)
- Homebrew installed on your macOS for dependencies

### Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/harperreed/dot-viewer.git
   cd dot-viewer
   ```

2. Ensure you have the required packages by running:
   ```bash
   brew install xcodegen bison flex
   ```

3. Build the app:
   ```bash
   make build
   ```

4. Open the project in Xcode:
   ```bash
   open DotViewer/DotViewer.xcodeproj
   ```

5. Run the app through Xcode to launch the Dot Viewer.

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
  └── .gitignore                   # Files to ignore in git
  ```

For detailed setup, workflow, and implementation notes, check the [documentation files](./docs) in this repository. If you encounter any issues, please raise them in the Issues section of this repository or create a pull request with your suggestions for improvements.

**Let's view the world of graphs! 🌍**
