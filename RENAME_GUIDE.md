# 🔄 Renaming Repository to EVA OS

## Current Status

The project has been rebranded from "Redox-EVA" to "EVA OS" in all documentation and code.

## Steps to Rename GitHub Repository

### 1. Rename on GitHub

1. Go to https://github.com/JoseRFJuniorLLMs/redox-EVA
2. Click **Settings**
3. In the **Repository name** field, change to: `EVA-OS`
4. Click **Rename**

GitHub will automatically:
- Redirect old URLs to new ones
- Update all links
- Preserve stars, forks, and issues

### 2. Update Local Repository

```bash
# Navigate to project
cd d:\dev\Redox-EVA

# Update remote URL
git remote set-url origin https://github.com/JoseRFJuniorLLMs/EVA-OS.git

# Verify
git remote -v

# Expected output:
# origin  https://github.com/JoseRFJuniorLLMs/EVA-OS.git (fetch)
# origin  https://github.com/JoseRFJuniorLLMs/EVA-OS.git (push)
```

### 3. Rename Local Directory (Optional)

```bash
# Go to parent directory
cd d:\dev

# Rename folder
mv Redox-EVA EVA-OS

# Navigate to new location
cd EVA-OS
```

### 4. Update Submodule (redox-EVA fork)

The `redox-EVA` submodule should also be renamed:

1. Go to https://github.com/JoseRFJuniorLLMs/redox-EVA (fork)
2. Rename to: `redox-os-eva`
3. Update `.gitmodules` in main repo:

```bash
# Edit .gitmodules
nano .gitmodules

# Change:
[submodule "redox-EVA"]
    path = redox-EVA
    url = https://github.com/JoseRFJuniorLLMs/redox-EVA.git

# To:
[submodule "redox-EVA"]
    path = redox-EVA
    url = https://github.com/JoseRFJuniorLLMs/redox-os-eva.git

# Update submodule
git submodule sync
git submodule update --init --recursive
```

### 5. Update All References

Files that reference the repository:

- [x] `README.md`
- [x] `eva-daemon/README.md`
- [x] `claude.md`
- [ ] `BUILD_EVA_OS.md` (to be created)
- [ ] `VERIFICATION.md`
- [ ] `PROJECT_SUMMARY.md`
- [ ] `fase1.md` through `fase4.md`

### 6. Push Changes

```bash
# Add all changes
git add .

# Commit
git commit -m "docs: Update all repository references to EVA-OS"

# Push to new repository name
git push origin main
```

---

## Verification Checklist

After renaming, verify:

- [ ] Repository accessible at new URL
- [ ] Old URL redirects to new URL
- [ ] Clone works with new URL
- [ ] All documentation updated
- [ ] Submodules working
- [ ] CI/CD still functioning (if applicable)
- [ ] README displays correctly
- [ ] Links in documentation work

---

## New Repository Structure

```
EVA-OS/
├── README.md                 # Main project README
├── LICENSE                   # MIT License
├── claude.md                 # Complete vision document
├── BUILD_EVA_OS.md          # Build instructions
├── PROJECT_SUMMARY.md        # Project summary
├── VERIFICATION.md           # Test results
├── fase1.md                  # Phase 1 documentation
├── fase2.md                  # Phase 2 documentation
├── fase3.md                  # Phase 3 documentation
├── fase4.md                  # Phase 4 documentation
├── eva-daemon/               # EVA daemon source code
│   ├── src/
│   ├── Cargo.toml
│   └── README.md
└── redox-EVA/                # Redox OS fork (submodule)
    ├── config/
    │   └── eva-os.toml       # EVA OS configuration
    └── recipes/
        └── other/
            └── eva-daemon/
```

---

## Benefits of Rename

1. **Clearer Identity** - "EVA OS" is more recognizable
2. **Better SEO** - Easier to find and remember
3. **Professional** - Sounds like a real OS project
4. **Shorter** - Easier to type and say
5. **Unique** - More distinctive name

---

## Communication

After rename, announce on:

- [ ] GitHub Discussions
- [ ] Project README
- [ ] Social media (if applicable)
- [ ] Related projects (EVA Mind, etc.)

**Example Announcement:**

> 🎉 **Big News!** We've rebranded to **EVA OS**!
> 
> The project formerly known as "Redox-EVA" is now officially **EVA OS** - The World's First Voice-Controlled Operating System.
> 
> All URLs will redirect automatically. Update your bookmarks to:
> https://github.com/JoseRFJuniorLLMs/EVA-OS
> 
> Same great project, better name! 🚀

---

**Status:** Ready for GitHub rename  
**Date:** 2026-02-04  
**Version:** 0.4.0
