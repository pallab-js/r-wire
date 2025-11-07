# Contributing to AuraCap

Thank you for your interest in contributing to AuraCap! This document provides guidelines and instructions for contributing.

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on constructive feedback
- Respect different viewpoints and experiences

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in [Issues](https://github.com/yourusername/r-wire/issues)
2. If not, create a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Node.js version, Rust version)
   - Screenshots if applicable

### Suggesting Features

1. Check existing feature requests in [Issues](https://github.com/yourusername/r-wire/issues)
2. Open a new issue with:
   - Clear description of the feature
   - Use case and motivation
   - Potential implementation approach (if you have ideas)

### Submitting Pull Requests

1. **Fork the repository** and clone your fork
2. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

3. **Make your changes**:
   - Follow the existing code style
   - Write clear, self-documenting code
   - Add comments for complex logic
   - Update documentation as needed

4. **Test your changes**:
   ```bash
   npm run check        # Type checking
   npm run tauri dev    # Test in development
   ```

5. **Commit your changes**:
   - Use clear, descriptive commit messages
   - Follow conventional commit format when possible:
     - `feat: add new feature`
     - `fix: resolve bug`
     - `docs: update documentation`
     - `refactor: improve code structure`
     - `test: add tests`

6. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

7. **Open a Pull Request**:
   - Provide a clear description of changes
   - Reference any related issues
   - Include screenshots for UI changes

## Development Setup

See the [README.md](README.md) for installation and setup instructions.

## Code Style

### TypeScript/Svelte
- Use TypeScript for type safety
- Follow Svelte best practices
- Use meaningful variable and function names
- Keep components focused and reusable

### Rust
- Follow Rust conventions and idioms
- Use `Result` types for error handling (avoid `unwrap()` in production code)
- Add documentation comments for public APIs
- Run `cargo fmt` and `cargo clippy` before committing

## Project Structure

- `src/` - Frontend SvelteKit application
- `src-tauri/` - Backend Rust/Tauri code
- `static/` - Static assets

## Testing

- Test your changes manually before submitting
- Ensure the application builds successfully
- Test on your target platform(s)
- Verify error handling works correctly

## Questions?

Feel free to open an issue for questions or reach out to maintainers.

Thank you for contributing to AuraCap! 🎉
