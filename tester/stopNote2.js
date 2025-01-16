const axios = require("axios");

// Base URL of your API server
const BASE_URL = "http://localhost:3000";

// Function to start the symphony
async function stopNote2() {
  try {
    const response = await axios.patch(`${BASE_URL}/api/v1/notes/Note2/stop`);

    console.log("Note2 stopped successfully:", response.data);
  } catch (error) {
    if (error.response) {
      console.error("Error response from server:", error.response);
    } else if (error.request) {
      console.error("No response received from server:", error.request);
    } else {
      console.error("Error setting up request:", error.message);
    }
  }
}

// Run the function
stopNote2();
