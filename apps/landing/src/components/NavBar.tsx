import clsx from 'clsx';
import React, { useEffect, useState } from 'react';
import { ReactComponent as AppLogo } from '../assets/app-logo.svg';

function NavLink(props: { link?: string; children: string }) {
  return (
    <a
      href={props.link ?? '#'}
      target="_blank"
      className="p-4 text-gray-300 no-underline transition cursor-pointer hover:text-gray-50"
    >
      {props.children}
    </a>
  );
}

export default function NavBar() {
  const [isAtTop, setIsAtTop] = useState(true);

  function onScroll(event: Event) {
    if (window.pageYOffset > 20) setIsAtTop(true);
    else if (isAtTop) setIsAtTop(false);
  }

  useEffect(() => {
    window.addEventListener('scroll', onScroll);
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  return (
    <div
      className={clsx(
        'fixed transition z-50 w-full h-16 border-b backdrop-blur',
        isAtTop ? 'border-gray-550 bg-gray-750 bg-opacity-80' : 'bg-transparent border-transparent'
      )}
    >
      <div className="container relative flex items-center h-full px-5 m-auto">
        <div className="absolute flex flex-row items-center">
          <AppLogo className="z-30 w-8 h-8 mr-3" />
          <h3 className="text-xl font-bold text-white">
            Spacedrive
            <span className="ml-2 text-xs text-gray-400 uppercase">BETA</span>
          </h3>
        </div>

        <div className="hidden m-auto space-x-4 text-white lg:block ">
          <NavLink link="https://github.com/spacedriveapp/#features">Features</NavLink>
          <NavLink link="https://github.com/spacedriveapp/spacedrive/tree/main/docs">Docs</NavLink>
          <NavLink link="https://github.com/spacedriveapp/spacedrive/blob/main/docs/product/faq.md">
            FAQ
          </NavLink>
          <NavLink link="https://github.com/spacedriveapp/spacedrive/tree/main/docs/changelog">
            Changelog
          </NavLink>
          <NavLink link="https://opencollective.com/spacedrive">Sponsor us</NavLink>
        </div>
      </div>
    </div>
  );
}
