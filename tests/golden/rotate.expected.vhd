library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_rotate_example is
  port (
    gl_p0_input : in unsigned(7 downto 0);
    gl_p1_rotated_left : out unsigned(7 downto 0);
    gl_p2_rotated_right : out unsigned(7 downto 0)
  );
end entity gl_m0_rotate_example;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_rotate_example is
  signal gl_s1_rotated_left : unsigned(7 downto 0);
  signal gl_s2_rotated_right : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_rotated_left <= rotate_left(gl_p0_input, 1);
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s2_rotated_right <= rotate_right(gl_p0_input, 1);
  end process gl_comb_1;
  gl_p1_rotated_left <= gl_s1_rotated_left;
  gl_p2_rotated_right <= gl_s2_rotated_right;
end architecture rtl;
