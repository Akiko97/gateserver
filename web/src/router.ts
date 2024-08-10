import { createWebHistory, createRouter } from 'vue-router'

import HomeView from './views/HomeView.vue'
import HanabiView from './views/HanabiView.vue'

const routes = [
  { path: '/', component: HomeView },
  { path: '/hanabi', component: HanabiView },
]

export const router = createRouter({
  history: createWebHistory(),
  routes,
})
