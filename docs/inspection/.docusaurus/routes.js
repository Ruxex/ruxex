import React from 'react';
import ComponentCreator from '@docusaurus/ComponentCreator';

export default [
  {
    path: '/',
    component: ComponentCreator('/', '90f'),
    routes: [
      {
        path: '/',
        component: ComponentCreator('/', '202'),
        routes: [
          {
            path: '/',
            component: ComponentCreator('/', 'd8e'),
            routes: [
              {
                path: '/intro/reading-order',
                component: ComponentCreator('/intro/reading-order', '14b'),
                exact: true,
                sidebar: "docsSidebar"
              },
              {
                path: '/',
                component: ComponentCreator('/', 'c48'),
                exact: true
              }
            ]
          }
        ]
      }
    ]
  },
  {
    path: '*',
    component: ComponentCreator('*'),
  },
];
