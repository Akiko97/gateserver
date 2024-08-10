<script setup lang="ts">
</script>

<template>
  <h1>Hello!</h1>
  <p>
    <strong>Current route path:</strong> {{ $route.fullPath }}
  </p>
  <nav>
    <RouterLink to="/">Home</RouterLink>
    <br>
    <RouterLink to="/hanabi">Hanabi Here!</RouterLink>
  </nav>
  <RouterView />
</template>

<style scoped>
</style>
